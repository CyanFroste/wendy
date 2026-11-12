use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, SizedSample, Stream, StreamConfig};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

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

    pub fn start<F>(&self, label: String, on_samples: F)
    where
        F: Fn(Vec<f32>) + Send + Sync + 'static,
    {
        let should_spawn = {
            let mut subs = self.subscribers.lock();
            subs.insert(label);
            self.generation.load(Ordering::SeqCst) == 0
        };

        if !should_spawn {
            return;
        }

        let this_gen = self.next_gen.fetch_add(1, Ordering::Relaxed);

        if self
            .generation
            .compare_exchange(0, this_gen, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        let generation = Arc::clone(&self.generation);
        let on_samples = Arc::new(on_samples);

        std::thread::spawn(move || {
            while generation.load(Ordering::SeqCst) == this_gen {
                let host = cpal::default_host();
                let Some(device) = host.default_output_device() else {
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                };

                let current_id = device.id().map(|id| id.to_string()).unwrap_or_default();
                let Ok(stream) = build_stream(&device, Arc::clone(&on_samples)) else {
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                };

                if stream.play().is_err() {
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }

                while generation.load(Ordering::SeqCst) == this_gen {
                    std::thread::sleep(Duration::from_secs(1));

                    let next_id = host
                        .default_output_device()
                        .and_then(|d| d.id().ok())
                        .map(|id| id.to_string())
                        .unwrap_or_default();

                    if next_id != current_id {
                        break;
                    }
                }
            }
        });
    }

    pub fn remove_subscriber(&self, label: &str) {
        let mut subscribers = self.subscribers.lock();
        subscribers.remove(label);

        if subscribers.is_empty() {
            self.generation.store(0, Ordering::SeqCst);
        }
    }
}

fn build_stream<F>(device: &Device, on_samples: Arc<F>) -> Result<Stream>
where
    F: Fn(Vec<f32>) + Send + Sync + 'static,
{
    let config = device.default_output_config()?;
    let format = config.sample_format();

    match format {
        SampleFormat::F32 => make_stream::<f32, F>(device, config.into(), on_samples),
        SampleFormat::I16 => make_stream::<i16, F>(device, config.into(), on_samples),
        SampleFormat::U16 => make_stream::<u16, F>(device, config.into(), on_samples),
        _ => Err(anyhow::anyhow!("unsupported format: {format:?}")),
    }
}

fn make_stream<T, F>(device: &Device, config: StreamConfig, on_samples: Arc<F>) -> Result<Stream>
where
    T: Sample<Float = f32> + SizedSample + Send + 'static,
    F: Fn(Vec<f32>) + Send + Sync + 'static,
{
    let channels = config.channels as usize;
    let mut last_emit = Instant::now();

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            if last_emit.elapsed() < Duration::from_millis(33) {
                return;
            }

            last_emit = Instant::now();
            let mono: Vec<f32> = data
                .chunks(channels)
                .map(|f| f.iter().map(|s| s.to_float_sample()).sum::<f32>() / channels as f32)
                .collect();

            on_samples(downsample(&mono, CHUNK_SIZE));
        },
        |err| eprintln!("audio stream error: {err}"),
        None,
    )?;

    Ok(stream)
}

fn downsample(samples: &[f32], target: usize) -> Vec<f32> {
    let len = samples.len();
    if len <= target {
        return samples.to_vec();
    }
    (0..target)
        .map(|i| {
            let chunk = &samples[(i * len / target)..((i + 1) * len / target).min(len)];
            let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
            let mean = chunk.iter().sum::<f32>() / chunk.len() as f32;
            if mean >= 0.0 { rms } else { -rms }
        })
        .collect()
}
