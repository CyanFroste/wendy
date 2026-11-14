use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, SizedSample, Stream, StreamConfig};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use volume_controls::{Volume, get_master_volume, set_master_volume, toggle_master_mute};

const CHUNK_SIZE: usize = 256;

pub struct AudioState {
    /// `0` = stopped; any non-zero value is the active capture generation ID.
    pub generation: Arc<AtomicU64>,
    pub subscribers: Mutex<HashSet<String>>,
    next_gen: AtomicU64,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            generation: Arc::new(AtomicU64::new(0)),
            subscribers: Mutex::new(HashSet::new()),
            next_gen: AtomicU64::new(1),
        }
    }

    pub fn start_capture<F>(&self, subscriber: String, on_update: F)
    where
        F: Fn(Vec<f32>) + Send + Sync + 'static,
    {
        let this_gen = {
            let mut subscribers = self.subscribers.lock();
            subscribers.insert(subscriber);

            if self.generation.load(Ordering::SeqCst) != 0 {
                return;
            }

            let id = self.next_gen.fetch_add(1, Ordering::Relaxed);
            self.generation.store(id, Ordering::SeqCst);

            id
        };

        let generation = Arc::clone(&self.generation);
        let on_update = Arc::new(on_update);

        std::thread::spawn(move || {
            while generation.load(Ordering::SeqCst) == this_gen {
                let host = cpal::default_host();
                let Some(device) = host.default_output_device() else {
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                };

                let current_id = device.id().map(|id| id.to_string()).unwrap_or_default();
                let (err_tx, err_rx) = mpsc::channel::<()>();

                let Ok(stream) = create_generic_stream(&device, Arc::clone(&on_update), err_tx)
                else {
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                };

                if stream.play().is_err() {
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }

                while generation.load(Ordering::SeqCst) == this_gen {
                    // wake immediately on buffer underrun / device reset, or poll device changes every 500ms
                    match err_rx.recv_timeout(Duration::from_millis(500)) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }

                    let next_id = host
                        .default_output_device()
                        .and_then(|d| d.id().ok())
                        .map(|id| id.to_string())
                        .unwrap_or_default();

                    if next_id != current_id {
                        break;
                    }
                }

                drop(stream); // explicitly release the dead WASAPI stream before recreating it
                if generation.load(Ordering::SeqCst) == this_gen {
                    std::thread::sleep(Duration::from_millis(150));
                }
            }
        });
    }

    pub fn remove_subscriber(&self, subscriber: &str) {
        let mut subscribers = self.subscribers.lock();
        subscribers.remove(subscriber);

        if subscribers.is_empty() {
            self.generation.store(0, Ordering::SeqCst);
        }
    }

    pub async fn get_master_volume(&self) -> Result<Volume> {
        Ok(tokio::task::spawn_blocking(get_master_volume).await??)
    }

    pub async fn set_master_volume(&self, level: f32) -> Result<Volume> {
        Ok(tokio::task::spawn_blocking(move || set_master_volume(level)).await??)
    }

    pub async fn toggle_master_mute(&self) -> Result<Volume> {
        Ok(tokio::task::spawn_blocking(toggle_master_mute).await??)
    }
}

fn create_generic_stream<F>(
    device: &Device,
    on_samples: Arc<F>,
    err_tx: mpsc::Sender<()>,
) -> Result<Stream>
where
    F: Fn(Vec<f32>) + Send + Sync + 'static,
{
    let config = device.default_output_config()?;
    let format = config.sample_format();

    match format {
        SampleFormat::F32 => create_stream::<f32, F>(device, config.into(), on_samples, err_tx),
        SampleFormat::I16 => create_stream::<i16, F>(device, config.into(), on_samples, err_tx),
        SampleFormat::U16 => create_stream::<u16, F>(device, config.into(), on_samples, err_tx),
        _ => Err(anyhow::anyhow!("unsupported format: {format:?}")),
    }
}

fn create_stream<T, F>(
    device: &Device,
    config: StreamConfig,
    on_samples: Arc<F>,
    err_tx: mpsc::Sender<()>,
) -> Result<Stream>
where
    T: Sample<Float = f32> + SizedSample + Send + 'static,
    F: Fn(Vec<f32>) + Send + Sync + 'static,
{
    let channels = (config.channels as usize).max(1);
    let (frame_tx, frame_rx) = mpsc::sync_channel::<([f32; CHUNK_SIZE], usize)>(1);
    let mut last_emit = Instant::now();

    std::thread::spawn(move || {
        while let Ok((buf, len)) = frame_rx.recv() {
            on_samples(buf[..len].to_vec());
        }
    });

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            if data.is_empty() || last_emit.elapsed() < Duration::from_millis(33) {
                return;
            }

            last_emit = Instant::now();
            let mut out = [0.0f32; CHUNK_SIZE];
            let len = downsample_interleaved(data, channels, &mut out);

            if len > 0 {
                let _ = frame_tx.try_send((out, len));
            }
        },
        move |err| {
            // non-fatal in WASAPI loopback (fires once during initial buffer priming)
            if matches!(err.kind(), cpal::ErrorKind::Xrun) {
                return;
            }
            eprintln!("audio stream error (restarting): {err}");
            let _ = err_tx.send(());
        },
        None,
    )?;

    Ok(stream)
}

fn downsample_interleaved<T>(data: &[T], channels: usize, out: &mut [f32; CHUNK_SIZE]) -> usize
where
    T: Sample<Float = f32> + SizedSample,
{
    let total_frames = data.len() / channels;
    if total_frames == 0 {
        return 0;
    }

    let buckets = total_frames.min(CHUNK_SIZE);
    let inv_channels = 1.0 / channels as f32;

    for (i, slot) in out.iter_mut().take(buckets).enumerate() {
        let start = i * total_frames / buckets;
        let end = ((i + 1) * total_frames / buckets).min(total_frames);
        let count = (end - start).max(1) as f32;
        let mut sum_sq = 0.0f32;
        let mut sum = 0.0f32;

        for f in start..end {
            let frame = &data[f * channels..(f + 1) * channels];
            let mono = frame.iter().map(|s| s.to_float_sample()).sum::<f32>() * inv_channels;
            sum_sq += mono * mono;
            sum += mono;
        }

        let rms = (sum_sq / count).sqrt();
        *slot = if sum >= 0.0 { rms } else { -rms };
    }

    buckets
}

pub mod volume_controls {
    use anyhow::Result;
    use serde::Serialize;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        IMMDeviceEnumerator, MMDeviceEnumerator, eMultimedia, eRender,
    };
    use windows::Win32::System::Com::{
        CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
    };
    use windows::core::GUID;

    #[derive(Serialize, Clone, Debug)]
    pub struct Volume {
        pub level: f32, // 0.0 ..= 1.0
        pub muted: bool,
    }

    struct ComScope(bool);

    impl ComScope {
        fn new() -> Self {
            let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            Self(hr.is_ok())
        }
    }

    impl Drop for ComScope {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    fn with_endpoint<F, R>(f: F) -> Result<R>
    where
        F: FnOnce(&IAudioEndpointVolume) -> windows::core::Result<R>,
    {
        let _com = ComScope::new();

        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;

            Ok(f(&endpoint)?)
        }
    }

    pub fn get_master_volume() -> Result<Volume> {
        with_endpoint(|ep| unsafe {
            Ok(Volume {
                level: ep.GetMasterVolumeLevelScalar()?,
                muted: ep.GetMute()?.as_bool(),
            })
        })
    }

    pub fn set_master_volume(level: f32) -> Result<Volume> {
        let clamped = level.clamp(0.0, 1.0);
        with_endpoint(|ep| unsafe {
            ep.SetMasterVolumeLevelScalar(clamped, &GUID::zeroed())?;

            // automatically unmute when user drags volume up above 0
            if clamped > 0.0 && ep.GetMute()?.as_bool() {
                ep.SetMute(false, &GUID::zeroed())?;
            }

            Ok(Volume {
                level: ep.GetMasterVolumeLevelScalar()?,
                muted: ep.GetMute()?.as_bool(),
            })
        })
    }

    pub fn toggle_master_mute() -> Result<Volume> {
        with_endpoint(|ep| unsafe {
            let next_mute = !ep.GetMute()?.as_bool();

            ep.SetMute(next_mute, &GUID::zeroed())?;

            Ok(Volume {
                level: ep.GetMasterVolumeLevelScalar()?,
                muted: next_mute,
            })
        })
    }
}
