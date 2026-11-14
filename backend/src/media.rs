use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    CurrentSessionChangedEventArgs, GlobalSystemMediaTransportControlsSession as MediaSession,
    GlobalSystemMediaTransportControlsSessionManager as MediaSessionManager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as MediaProperties,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
    MediaPropertiesChangedEventArgs, PlaybackInfoChangedEventArgs, SessionsChangedEventArgs,
    TimelinePropertiesChangedEventArgs,
};
use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};
use windows::System::AppDiagnosticInfo;
use windows::core::HSTRING;

#[derive(Default)]
pub struct MediaState {
    manager: OnceCell<MediaSessionManager>,
    pub listener: Mutex<Option<MediaListener>>,
    pub subscribers: Mutex<HashSet<String>>,
}

impl MediaState {
    pub async fn manager(&self) -> Result<&MediaSessionManager> {
        self.manager
            .get_or_try_init(|| async { Ok(MediaSessionManager::RequestAsync()?.await?) })
            .await
    }

    pub async fn get_active_media(&self) -> Result<Option<MediaSessionInfo>> {
        let manager = self.manager().await?;

        if let Ok(current) = manager.GetCurrentSession()
            && let Some(info) = parse_session(&current, true).await
        {
            return Ok(Some(info));
        }

        if let Ok(sessions) = manager.GetSessions() {
            for session in sessions {
                if let Some(info) = parse_session(&session, false).await {
                    return Ok(Some(info));
                }
            }
        }

        Ok(None)
    }

    pub async fn send_command(&self, player_id: String, cmd: MediaCommand) -> Result<()> {
        let manager = self.manager().await?;

        let session = manager
            .GetSessions()?
            .into_iter()
            .find(|s| {
                s.SourceAppUserModelId()
                    .map(|id| id == player_id)
                    .unwrap_or(false)
            })
            .or_else(|| manager.GetCurrentSession().ok())
            .ok_or_else(|| anyhow::anyhow!("No active media session found"))?;

        match cmd {
            MediaCommand::Play => {
                session.TryPlayAsync()?.await?;
            }
            MediaCommand::Pause => {
                session.TryPauseAsync()?.await?;
            }
            MediaCommand::Toggle => {
                session.TryTogglePlayPauseAsync()?.await?;
            }
            MediaCommand::Next => {
                session.TrySkipNextAsync()?.await?;
            }
            MediaCommand::Prev => {
                session.TrySkipPreviousAsync()?.await?;
            }
            MediaCommand::Seek { position_ms } => {
                let ticks = (Duration::from_millis(position_ms).as_nanos() / 100) as i64;

                session.TryChangePlaybackPositionAsync(ticks)?.await?;
            }
        }

        Ok(())
    }

    pub fn remove_subscriber(&self, subscriber: &str) {
        let should_stop = {
            let mut subscribers = self.subscribers.lock();
            subscribers.remove(subscriber);
            subscribers.is_empty()
        };

        if should_stop && let Some(listener) = self.listener.lock().take() {
            tokio::task::spawn_blocking(move || listener.stop());
        }
    }

    pub async fn start_listener<F>(&self, subscriber: String, on_update: F) -> Result<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let is_started = {
            let mut subscribers = self.subscribers.lock();
            subscribers.insert(subscriber);
            self.listener.lock().is_some()
        };

        if is_started {
            return Ok(());
        }

        let rt = tokio::runtime::Handle::current();

        let listener =
            tokio::task::spawn_blocking(move || rt.block_on(MediaListener::start(on_update)))
                .await??;

        let listener = {
            let subscribers = self.subscribers.lock();
            let mut guard = self.listener.lock();

            if !subscribers.is_empty() && guard.is_none() {
                *guard = Some(listener);

                None
            } else {
                Some(listener)
            }
        };

        if let Some(listener) = listener {
            tokio::task::spawn_blocking(move || listener.stop());
        }

        Ok(())
    }
}

pub struct MediaListener {
    manager: MediaSessionManager,
    sessions_changed_token: i64,
    current_changed_token: i64,
    sessions: Arc<Mutex<HashMap<String, SessionEntry>>>,
}

impl MediaListener {
    pub async fn start<F>(on_update: F) -> Result<Self>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let manager = MediaSessionManager::RequestAsync()?.await?;
        let debouncer = TrailingDebouncer::new(on_update);
        let sessions: Arc<Mutex<HashMap<String, SessionEntry>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // listen for new or closed media sessions
        let debouncer_clone = debouncer.clone();
        let sessions_clone = Arc::clone(&sessions);
        let sessions_changed_token = manager.SessionsChanged(&TypedEventHandler::<
            MediaSessionManager,
            SessionsChangedEventArgs,
        >::new(move |mgr, _| {
            if let Some(mgr) = mgr.as_ref() {
                sync_sessions(mgr, &debouncer_clone, &mut sessions_clone.lock());
            }
            debouncer_clone.trigger();
            Ok(())
        }))?;

        // listen for active session switches (e.g., pausing Spotify and playing YouTube)
        let debouncer_clone = debouncer.clone();
        let current_changed_token =
            manager.CurrentSessionChanged(&TypedEventHandler::<
                MediaSessionManager,
                CurrentSessionChangedEventArgs,
            >::new(move |_, _| {
                debouncer_clone.trigger();
                Ok(())
            }))?;

        // attach listeners to already-open sessions
        sync_sessions(&manager, &debouncer, &mut sessions.lock());

        Ok(Self {
            manager,
            sessions_changed_token,
            current_changed_token,
            sessions,
        })
    }

    pub fn stop(self) {
        let _ = self
            .manager
            .RemoveSessionsChanged(self.sessions_changed_token);
        let _ = self
            .manager
            .RemoveCurrentSessionChanged(self.current_changed_token);

        for (_, entry) in self.sessions.lock().drain() {
            let _ = entry
                .session
                .RemoveMediaPropertiesChanged(entry.metadata_token);
            let _ = entry
                .session
                .RemovePlaybackInfoChanged(entry.playback_token);
            let _ = entry
                .session
                .RemoveTimelinePropertiesChanged(entry.timeline_token);
        }
    }
}

/// Trailing-edge debouncer so rapid bursts of WinRT events settle before notifying the UI.
#[derive(Clone)]
struct TrailingDebouncer {
    pending: Arc<Mutex<bool>>,
    rt: tokio::runtime::Handle,
    on_emit: Arc<dyn Fn() + Send + Sync>,
}

impl TrailingDebouncer {
    fn new<F>(on_emit: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            pending: Arc::new(Mutex::new(false)),
            rt: tokio::runtime::Handle::current(),
            on_emit: Arc::new(on_emit),
        }
    }

    fn trigger(&self) {
        let mut pending = self.pending.lock();
        if *pending {
            return;
        }

        *pending = true;
        drop(pending);

        let pending = Arc::clone(&self.pending);
        let on_emit = Arc::clone(&self.on_emit);

        self.rt.spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;

            *pending.lock() = false;
            on_emit();
        });
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MediaTimeline {
    pub start_ms: u64,
    pub end_ms: u64,
    pub position_ms: u64,
    pub can_seek: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MediaControls {
    pub can_play: bool,
    pub can_pause: bool,
    pub can_toggle: bool,
    pub can_next: bool,
    pub can_prev: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MediaSessionInfo {
    pub player_id: String,
    pub player_name: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
    pub is_current: bool,
    pub thumbnail: Vec<u8>,
    pub controls: MediaControls,
    pub timeline: Option<MediaTimeline>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum MediaCommand {
    Play,
    Pause,
    Toggle,
    Next,
    Prev,
    #[serde(rename_all = "camelCase")]
    Seek {
        position_ms: u64,
    },
}

struct SessionEntry {
    session: MediaSession,
    metadata_token: i64,
    playback_token: i64,
    timeline_token: i64,
}

fn sync_sessions(
    manager: &MediaSessionManager,
    debouncer: &TrailingDebouncer,
    map: &mut HashMap<String, SessionEntry>,
) {
    let Ok(raw_sessions) = manager.GetSessions() else {
        return;
    };

    let mut active_ids = HashSet::new();

    for session in raw_sessions {
        let Ok(id_hstr) = session.SourceAppUserModelId() else {
            continue;
        };

        let id = id_hstr.to_string();
        active_ids.insert(id.clone());

        if map.contains_key(&id) {
            continue;
        }

        let debouncer_clone = debouncer.clone();
        let Ok(metadata_token) = session.MediaPropertiesChanged(&TypedEventHandler::<
            MediaSession,
            MediaPropertiesChangedEventArgs,
        >::new(move |_, _| {
            debouncer_clone.trigger();
            Ok(())
        })) else {
            continue;
        };

        let debouncer_clone = debouncer.clone();
        let Ok(playback_token) = session.PlaybackInfoChanged(&TypedEventHandler::<
            MediaSession,
            PlaybackInfoChangedEventArgs,
        >::new(move |_, _| {
            debouncer_clone.trigger();
            Ok(())
        })) else {
            let _ = session.RemoveMediaPropertiesChanged(metadata_token);
            continue;
        };

        let debouncer_clone = debouncer.clone();
        let Ok(timeline_token) = session.TimelinePropertiesChanged(&TypedEventHandler::<
            MediaSession,
            TimelinePropertiesChangedEventArgs,
        >::new(move |_, _| {
            debouncer_clone.trigger();
            Ok(())
        })) else {
            let _ = session.RemoveMediaPropertiesChanged(metadata_token);
            let _ = session.RemovePlaybackInfoChanged(playback_token);
            continue;
        };

        map.insert(
            id,
            SessionEntry {
                session,
                metadata_token,
                playback_token,
                timeline_token,
            },
        );
    }

    // remove stale sessions that have closed
    let stale: Vec<String> = map
        .keys()
        .filter(|id| !active_ids.contains(*id))
        .cloned()
        .collect();

    for id in stale {
        if let Some(entry) = map.remove(&id) {
            let _ = entry
                .session
                .RemoveMediaPropertiesChanged(entry.metadata_token);
            let _ = entry
                .session
                .RemovePlaybackInfoChanged(entry.playback_token);
            let _ = entry
                .session
                .RemoveTimelinePropertiesChanged(entry.timeline_token);
        }
    }
}

async fn resolve_player_name(app_id: &str) -> String {
    if let Ok(op) = AppDiagnosticInfo::RequestInfoForAppUserModelId(&HSTRING::from(app_id))
        && let Ok(iter) = op.await
        && let Some(diag) = iter.into_iter().next()
        && let Ok(name) = diag
            .AppInfo()
            .and_then(|a| a.DisplayInfo())
            .and_then(|d| d.DisplayName())
    {
        let s = name.to_string_lossy();

        if !s.is_empty() {
            return s;
        }
    }

    let lower = app_id.to_lowercase();
    if lower.contains("spotify") {
        "Spotify".into()
    } else if lower.contains("chrome") {
        "Google Chrome".into()
    } else if lower.contains("msedge") {
        "Microsoft Edge".into()
    } else if lower.contains("firefox") {
        "Firefox".into()
    } else if lower.contains("brave") {
        "Brave".into()
    } else {
        app_id
            .strip_suffix(".exe")
            .or_else(|| app_id.strip_suffix(".EXE"))
            .unwrap_or(app_id)
            .to_string()
    }
}

async fn read_thumbnail(props: &MediaProperties) -> Vec<u8> {
    let Ok(thumb_ref) = props.Thumbnail() else {
        return Vec::new();
    };
    let Ok(stream) = async { thumb_ref.OpenReadAsync()?.await }.await else {
        return Vec::new();
    };
    let Ok(size) = stream.Size() else {
        return Vec::new();
    };

    if size == 0 || size > 10_000_000 {
        return Vec::new();
    }

    let len = size as u32;
    let Ok(buffer) = Buffer::Create(len) else {
        return Vec::new();
    };

    if async {
        stream
            .ReadAsync(&buffer, len, InputStreamOptions::ReadAhead)?
            .await
    }
    .await
    .is_err()
    {
        return Vec::new();
    }

    let Ok(reader) = DataReader::FromBuffer(&buffer) else {
        return Vec::new();
    };
    let mut bytes = vec![0u8; len as usize];

    if reader.ReadBytes(&mut bytes).is_err() {
        return Vec::new();
    }

    bytes
}

async fn parse_session(session: &MediaSession, is_current: bool) -> Option<MediaSessionInfo> {
    let player_id = session.SourceAppUserModelId().ok()?.to_string();
    let props = session.TryGetMediaPropertiesAsync().ok()?.await.ok()?;
    let playback = session.GetPlaybackInfo().ok()?;
    let ctrl = playback.Controls().ok()?;

    let is_playing = playback
        .PlaybackStatus()
        .map(|s| s == PlaybackStatus::Playing)
        .unwrap_or(false);

    let timeline = session.GetTimelineProperties().ok().and_then(|tl| {
        let start = (tl.StartTime().ok()?.Duration.max(0) / 10_000) as u64;
        let end = (tl.EndTime().ok()?.Duration.max(0) / 10_000) as u64;
        let pos = (tl.Position().ok()?.Duration.max(0) / 10_000) as u64;
        let can_seek = ctrl.IsPlaybackPositionEnabled().unwrap_or(false);

        if end == 0 && pos == 0 && !can_seek {
            return None;
        }

        Some(MediaTimeline {
            start_ms: start,
            end_ms: end.max(pos),
            position_ms: pos,
            can_seek,
        })
    });

    Some(MediaSessionInfo {
        player_name: resolve_player_name(&player_id).await,
        player_id,
        title: props.Title().map(|s| s.to_string()).unwrap_or_default(),
        artist: props.Artist().map(|s| s.to_string()).unwrap_or_default(),
        album: props
            .AlbumTitle()
            .map(|s| s.to_string())
            .unwrap_or_default(),
        is_playing,
        is_current,
        thumbnail: read_thumbnail(&props).await,
        controls: MediaControls {
            can_play: ctrl.IsPlayEnabled().unwrap_or(false),
            can_pause: ctrl.IsPauseEnabled().unwrap_or(false),
            can_toggle: ctrl.IsPlayPauseToggleEnabled().unwrap_or(false),
            can_next: ctrl.IsNextEnabled().unwrap_or(false),
            can_prev: ctrl.IsPreviousEnabled().unwrap_or(false),
        },
        timeline,
    })
}
