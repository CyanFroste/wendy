use crate::audio::AudioState;
use crate::types::{InstalledApp, MediaState, Widget};
use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::fs::File;
use std::path::PathBuf;
use std::{env, fs};
use tauri::{Manager, Window, WindowEvent};
use walkdir::WalkDir;

pub mod constants {
    pub const RPC_SYSTEM_GET_CPU_INFO: &str = "system.get_cpu_info";
    pub const RPC_SYSTEM_GET_RAM_INFO: &str = "system.get_ram_info";
    pub const RPC_SYSTEM_GET_DISK_INFO: &str = "system.get_disk_info";
    pub const RPC_SYSTEM_GET_INSTALLED_APPS: &str = "system.get_installed_apps";
    pub const RPC_SYSTEM_OPEN_INSTALLED_APP: &str = "system.open_installed_app";
    pub const RPC_MEDIA_GET_ACTIVE_MEDIA: &str = "media.get_active_media";
    pub const RPC_MEDIA_SEND_COMMAND: &str = "media.send_command";
    pub const RPC_AUDIO_GET_MASTER_VOLUME: &str = "audio.get_master_volume";
    pub const RPC_AUDIO_SET_MASTER_VOLUME: &str = "audio.set_master_volume";
    pub const RPC_AUDIO_TOGGLE_MASTER_MUTE: &str = "audio.toggle_master_mute";
    pub const RPC_SYSTEM_OPEN_RECYCLE_BIN: &str = "system.open_recycle_bin";
    pub const RPC_SYSTEM_GET_RECYCLE_BIN_COUNT: &str = "system.get_recycle_bin_count";
}

pub fn get_rpc_required_scopes(method: &str) -> Option<&'static [&'static str]> {
    use constants::*;

    match method {
        RPC_SYSTEM_GET_CPU_INFO => Some(&["system:read"]),
        RPC_SYSTEM_GET_RAM_INFO => Some(&["system:read"]),
        RPC_SYSTEM_GET_DISK_INFO => Some(&["system:read"]),
        RPC_SYSTEM_GET_INSTALLED_APPS => Some(&["system:read"]),
        RPC_SYSTEM_OPEN_INSTALLED_APP => Some(&["system:exec"]),
        RPC_MEDIA_GET_ACTIVE_MEDIA => Some(&["media:read"]),
        RPC_MEDIA_SEND_COMMAND => Some(&["media:exec"]),
        RPC_AUDIO_GET_MASTER_VOLUME => Some(&["audio:read"]),
        RPC_AUDIO_SET_MASTER_VOLUME => Some(&["audio:write"]),
        RPC_AUDIO_TOGGLE_MASTER_MUTE => Some(&["audio:write"]),
        RPC_SYSTEM_OPEN_RECYCLE_BIN => Some(&["system:exec"]),
        RPC_SYSTEM_GET_RECYCLE_BIN_COUNT => Some(&["system:read"]),
        _ => None,
    }
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::Destroyed = event {
        let media_state = window.state::<MediaState>();
        let listener = {
            let mut subscribers = media_state.subscribers.lock();
            subscribers.remove(window.label());

            if subscribers.is_empty() {
                media_state.listener.lock().take()
            } else {
                None
            }
        };

        if let Some(listener) = listener {
            tauri::async_runtime::spawn_blocking(move || listener.stop());
        }

        window
            .state::<AudioState>()
            .remove_subscriber(window.label());
    }
}

pub async fn get_installed_apps() -> Result<Vec<InstalledApp>> {
    let mut apps = Vec::new();
    let mut search_dirs = Vec::new();

    if let Ok(var) = env::var("ProgramData") {
        search_dirs.push(PathBuf::from(var).join(r"Microsoft\Windows\Start Menu\Programs"));
    }

    if let Ok(var) = env::var("APPDATA") {
        search_dirs.push(PathBuf::from(var).join(r"Microsoft\Windows\Start Menu\Programs"));
    }

    for dir in search_dirs {
        for entry in WalkDir::new(dir).into_iter().filter_map(|x| x.ok()) {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("lnk") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let lower = name.to_lowercase();
                if lower.contains("uninstall") || lower.contains("help") {
                    continue;
                }

                let icon_base64 = get_app_icon(path.to_string_lossy()).await?;
                let path = path.to_path_buf();

                apps.push(InstalledApp {
                    name,
                    path,
                    icon_base64,
                });
            }
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));

    Ok(apps)
}

pub async fn get_app_icon(path: impl AsRef<str>) -> Result<Option<String>> {
    let path = path.as_ref().to_string();

    let res = tokio::task::spawn_blocking(move || {
        systemicons::get_icon(&path, 32)
            .ok()
            .map(|bytes| format!("data:image/png;base64,{}", BASE64.encode(bytes)))
    })
    .await?;

    Ok(res)
}

pub fn get_widgets(widgets_path: PathBuf) -> Result<Vec<Widget>> {
    let mut widgets = Vec::new();

    for entry in widgets_path.read_dir()? {
        let entry = entry?;
        let manifest_path = entry.path().join("manifest.json");

        if manifest_path.exists() {
            let mut widget: Widget = serde_json::from_reader(File::open(&manifest_path)?)?;

            widget.path = entry.path();
            widget.preview_path = widget.preview_path.map(|x| entry.path().join(x));
            widgets.push(widget);
        }
    }

    Ok(widgets)
}

pub fn get_recycle_bin_count() -> usize {
    let mut count: usize = 0;

    for drive in b'A'..=b'Z' {
        let bin_root = PathBuf::from(format!("{}:\\$Recycle.Bin", drive as char));
        let Ok(sid_dirs) = fs::read_dir(bin_root) else {
            continue;
        };

        for sid in sid_dirs.flatten() {
            let sid_path = sid.path();
            let Ok(entries) = fs::read_dir(&sid_path) else {
                continue;
            };

            count += entries
                .flatten()
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .and_then(|n| n.strip_prefix("$I"))
                        .is_some_and(|suffix| sid_path.join(format!("$R{suffix}")).exists())
                })
                .count();
        }
    }

    count
}
