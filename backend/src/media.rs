use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State as AppState};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    CurrentSessionChangedEventArgs, GlobalSystemMediaTransportControlsSession as MediaSession,
    GlobalSystemMediaTransportControlsSessionManager as MediaSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
    MediaPropertiesChangedEventArgs, PlaybackInfoChangedEventArgs, SessionsChangedEventArgs,
    TimelinePropertiesChangedEventArgs,
};
use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};
use windows::System::AppDiagnosticInfo;
use windows::core::HSTRING;

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
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MediaCommand {
    Play,
    Pause,
    Toggle,
    Next,
    Prev,
    Seek { position_ms: u64 },
}

/// Trailing-edge debouncer so rapid bursts of WinRT events settle before notifying the UI.
#[derive(Clone)]
struct TrailingDebouncer {
    pending: Arc<Mutex<bool>>,
}

impl TrailingDebouncer {
    fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(false)),
        }
    }

    fn trigger(&self, app: AppHandle) {
        let mut pending = self.pending.lock();

        if *pending {
            return; // timer already scheduled, it will fire after the burst settles
        }

        *pending = true;
        drop(pending);

        let pending = Arc::clone(&self.pending);

        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;

            *pending.lock() = false;
            let _ = app.emit("media_updated", ());
        });
    }
}

struct SessionEntry {
    session: MediaSession,
    metadata_token: i64,
    playback_token: i64,
    timeline_token: i64,
}

pub struct WinMediaListener {
    manager: MediaSessionManager,
    sessions_changed_token: i64,
    current_changed_token: i64,
    sessions: Arc<Mutex<HashMap<String, SessionEntry>>>,
}

impl WinMediaListener {
    pub async fn start(app: AppHandle) -> Result<Self> {
        let manager = MediaSessionManager::RequestAsync()?.await?;
        let debouncer = TrailingDebouncer::new();
        let sessions: Arc<Mutex<HashMap<String, SessionEntry>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // 1. Listen for new or closed media sessions
        let d1 = debouncer.clone();
        let a1 = app.clone();
        let s1 = Arc::clone(&sessions);
        let sessions_changed_token = manager.SessionsChanged(&TypedEventHandler::<
            MediaSessionManager,
            SessionsChangedEventArgs,
        >::new(move |mgr, _| {
            if let Some(mgr) = mgr.as_ref() {
                sync_sessions(mgr, &a1, &d1, &mut s1.lock());
            }
            d1.trigger(a1.clone());
            Ok(())
        }))?;

        // 2. Listen for active session switches (e.g., pausing Spotify and playing YouTube)
        let d2 = debouncer.clone();
        let a2 = app.clone();
        let current_changed_token =
            manager.CurrentSessionChanged(&TypedEventHandler::<
                MediaSessionManager,
                CurrentSessionChangedEventArgs,
            >::new(move |_, _| {
                d2.trigger(a2.clone());
                Ok(())
            }))?;

        // 3. Attach listeners to already-open sessions
        sync_sessions(&manager, &app, &debouncer, &mut sessions.lock());

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

fn sync_sessions(
    manager: &MediaSessionManager,
    app: &AppHandle,
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

        let d_meta = debouncer.clone();
        let a_meta = app.clone();
        let Ok(metadata_token) = session.MediaPropertiesChanged(&TypedEventHandler::<
            MediaSession,
            MediaPropertiesChangedEventArgs,
        >::new(move |_, _| {
            d_meta.trigger(a_meta.clone());
            Ok(())
        })) else {
            continue;
        };

        let d_play = debouncer.clone();
        let a_play = app.clone();
        let Ok(playback_token) = session.PlaybackInfoChanged(&TypedEventHandler::<
            MediaSession,
            PlaybackInfoChangedEventArgs,
        >::new(move |_, _| {
            d_play.trigger(a_play.clone());
            Ok(())
        })) else {
            let _ = session.RemoveMediaPropertiesChanged(metadata_token);
            continue;
        };

        let d_time = debouncer.clone();
        let a_time = app.clone();
        let Ok(timeline_token) = session.TimelinePropertiesChanged(&TypedEventHandler::<
            MediaSession,
            TimelinePropertiesChangedEventArgs,
        >::new(move |_, _| {
            d_time.trigger(a_time.clone());
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

    // Remove stale sessions that have closed
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
    if let Ok(op) = AppDiagnosticInfo::RequestInfoForAppUserModelId(&HSTRING::from(app_id)) {
        if let Ok(iter) = op.await {
            if let Some(diag) = iter.into_iter().next() {
                if let Ok(name) = diag
                    .AppInfo()
                    .and_then(|a| a.DisplayInfo())
                    .and_then(|d| d.DisplayName())
                {
                    let s = name.to_string_lossy();
                    if !s.is_empty() {
                        return s;
                    }
                }
            }
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

async fn read_thumbnail(
    props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
) -> Vec<u8> {
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
        if end == 0 {
            return None;
        }
        Some(MediaTimeline {
            start_ms: start,
            end_ms: end,
            position_ms: pos,
            can_seek: ctrl.IsPlaybackPositionEnabled().unwrap_or(false),
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

pub async fn fetch_active_media() -> Result<Option<MediaSessionInfo>> {
    let manager = MediaSessionManager::RequestAsync()?.await?;

    if let Ok(current) = manager.GetCurrentSession() {
        if let Some(info) = parse_session(&current, true).await {
            return Ok(Some(info));
        }
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

pub async fn send_command(player_id: String, cmd: MediaCommand) -> Result<()> {
    let manager = MediaSessionManager::RequestAsync()?.await?;

    let session = manager
        .GetSessions()?
        .into_iter()
        .find(|s| {
            s.SourceAppUserModelId()
                .map(|id| id.to_string() == player_id)
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

#[derive(Default)]
pub struct MediaState {
    listener: Mutex<Option<WinMediaListener>>,
}

#[tauri::command]
pub async fn start_media_listener(
    app: AppHandle,
    state: AppState<'_, MediaState>,
) -> Result<(), crate::types::Error> {
    if state.listener.lock().is_some() {
        return Ok(());
    }

    let listener = WinMediaListener::start(app).await?;
    let mut guard = state.listener.lock();

    if guard.is_none() {
        *guard = Some(listener);
    } else {
        listener.stop();
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_media_listener(
    state: AppState<'_, MediaState>,
) -> Result<(), crate::types::Error> {
    if let Some(listener) = state.listener.lock().take() {
        tokio::task::spawn_blocking(move || listener.stop())
            .await
            .map_err(crate::types::Error::from_error)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_active_media() -> Result<Option<MediaSessionInfo>, crate::types::Error> {
    let res = fetch_active_media().await?;

    Ok(res)
}

#[tauri::command]
pub async fn send_media_command(
    player_id: String,
    command: MediaCommand,
) -> Result<(), crate::types::Error> {
    send_command(player_id, command).await?;

    Ok(())
}
