use crate::media::{MediaCommand, MediaListener, get_active_media, send_media_command};
use crate::types::{
    CacheState, CpuInfo, DiskInfo, Error, MediaState, RamInfo, SystemState, Widget,
};
use crate::utils;
use crate::utils::{
    RPC_MEDIA_GET_ACTIVE_MEDIA, RPC_MEDIA_SEND_COMMAND, RPC_SYSTEM_GET_CPU_INFO,
    RPC_SYSTEM_GET_DISK_INFO, RPC_SYSTEM_GET_INSTALLED_APPS, RPC_SYSTEM_GET_RAM_INFO,
    RPC_SYSTEM_OPEN_INSTALLED_APP,
};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use sysinfo::Disks;
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager, State as AppState};
use tauri_plugin_fs::FsExt;

#[tauri::command]
pub async fn start_media_listener(
    app_handle: AppHandle,
    media_state: AppState<'_, MediaState>,
    window_label: String,
) -> Result<(), Error> {
    let is_started = {
        let mut subscribers = media_state.subscribers.lock();
        subscribers.insert(window_label);
        media_state.listener.lock().is_some()
    };

    if is_started {
        return Ok(());
    }

    let app_handle = app_handle.clone();
    let rt = tokio::runtime::Handle::current();

    let listener = tokio::task::spawn_blocking(move || {
        rt.block_on(MediaListener::start(move || {
            let rt = tokio::runtime::Handle::current();
            let app_handle = app_handle.clone();

            tokio::task::spawn_blocking(move || {
                let subscribers: Vec<String> = app_handle
                    .state::<MediaState>()
                    .subscribers
                    .lock()
                    .iter()
                    .cloned()
                    .collect();

                if subscribers.is_empty() {
                    return;
                }

                if let Ok(data) = rt.block_on(get_active_media()) {
                    for subscriber in subscribers {
                        let _ = app_handle.emit_to(&subscriber, "media-updated", &data);
                    }
                }
            });
        }))
    })
    .await
    .map_err(Error::from_error)??;

    let subscribers: Vec<String> = media_state.subscribers.lock().iter().cloned().collect();
    let mut guard = media_state.listener.lock();

    if subscribers.is_empty() {
        listener.stop();
    } else if guard.is_none() {
        *guard = Some(listener);
    } else {
        listener.stop();
    }

    Ok(())
}

#[tauri::command]
pub async fn widget_rpc(
    cache_state: AppState<CacheState, '_>,
    system_state: AppState<SystemState, '_>,
    widget_id: String,
    method: String,
    params: JsonValue,
) -> Result<JsonValue, Error> {
    let widget = cache_state
        .widgets_cache
        .read()
        .get(&widget_id)
        .cloned()
        .ok_or_else(|| Error::new("widget not found"))?;

    if utils::get_rpc_required_scopes(&method).is_some_and(|req| {
        req.iter()
            .any(|&x| !widget.permissions.iter().any(|s| s == x))
    }) {
        return Err(Error::new("permission denied"));
    }

    match method.as_str() {
        RPC_MEDIA_GET_ACTIVE_MEDIA => {
            let rt = tokio::runtime::Handle::current();
            let data = tokio::task::spawn_blocking(move || rt.block_on(get_active_media()))
                .await
                .map_err(Error::from_error)??;

            Ok(serde_json::to_value(&data)?)
        }

        RPC_MEDIA_SEND_COMMAND => {
            let player_id = params["playerId"]
                .as_str()
                .ok_or_else(|| Error::new("missing parameter 'playerId'"))?
                .to_string();
            let command: MediaCommand = serde_json::from_value(params["command"].clone())?;

            send_media_command(player_id, command).await?;

            Ok(JsonValue::Null)
        }

        RPC_SYSTEM_GET_CPU_INFO => {
            let mut system = system_state.system.lock();

            system.refresh_cpu_all();

            let cpus = system.cpus();
            let data = CpuInfo {
                brand: cpus
                    .first()
                    .map(|c| c.brand().to_string())
                    .unwrap_or_default(),
                global_usage: system.global_cpu_usage(),
                core_count: cpus.len(),
                core_usages: cpus.iter().map(|c| c.usage()).collect(),
            };

            Ok(serde_json::to_value(&data)?)
        }

        RPC_SYSTEM_GET_RAM_INFO => {
            let mut system = system_state.system.lock();

            system.refresh_memory();

            let data = RamInfo {
                total_bytes: system.total_memory(),
                used_bytes: system.used_memory(),
                available_bytes: system.available_memory(),
                swap_total_bytes: system.total_swap(),
                swap_used_bytes: system.used_swap(),
            };

            Ok(serde_json::to_value(&data)?)
        }

        RPC_SYSTEM_GET_DISK_INFO => {
            let disks = Disks::new_with_refreshed_list()?;
            let data: Vec<DiskInfo> = disks
                .iter()
                .map(|disk| DiskInfo {
                    name: disk.name().to_string_lossy().to_string(),
                    mount_point: disk.mount_point().to_string_lossy().to_string(),
                    file_system: disk.file_system().to_string_lossy().to_string(),
                    total_bytes: disk.total_space(),
                    available_bytes: disk.available_space(),
                    is_removable: disk.is_removable(),
                })
                .collect();

            Ok(serde_json::to_value(&data)?)
        }

        RPC_SYSTEM_GET_INSTALLED_APPS => Ok(serde_json::to_value(&cache_state.installed_apps)?),

        RPC_SYSTEM_OPEN_INSTALLED_APP => {
            let path = params["path"]
                .as_str()
                .ok_or_else(|| Error::new("missing parameter 'path'"))?;
            let path = PathBuf::from(path);

            if !cache_state.installed_apps.iter().any(|x| x.path == path) {
                return Err(Error::new("invalid application path"));
            }

            // ! NOTE: need to make sure this doesn't make the launched app bound to the current process
            open::that_detached(path)?;

            Ok(JsonValue::Null)
        }

        _ => Err(Error::new("unknown method")),
    }
}

#[tauri::command]
pub async fn get_widget_asset(
    cache_state: AppState<CacheState, '_>,
    widget_id: String,
    path: String,
) -> Result<Response, Error> {
    // only getting for widgets from cache for now
    let Some(widget_path) = cache_state
        .widgets_cache
        .read()
        .get(&widget_id)
        .map(|x| x.path.clone())
    else {
        return Err(Error::new("widget not found"));
    };

    let path: PathBuf = Path::new(&path)
        .components()
        .filter_map(|x| match x {
            Component::Normal(s) => Some(s),
            _ => None,
        })
        .collect();

    let asset_path = widget_path.join("assets").join(path);

    if !asset_path.exists() || !asset_path.is_file() {
        return Err(Error::new("asset not found"));
    }

    let bytes = fs::read(&asset_path)?;
    let res = Response::new(bytes);

    Ok(res)
}

#[tauri::command]
pub fn get_widgets(
    cache_state: AppState<CacheState, '_>,
    widgets_path: PathBuf,
    refetch: bool,
) -> Result<Vec<Widget>, Error> {
    let guard = cache_state.widgets_cache.read();

    if !refetch && !guard.is_empty() {
        return Ok(guard.values().cloned().collect());
    }

    drop(guard);

    let widgets = utils::get_widgets(widgets_path)?;

    *cache_state.widgets_cache.write() =
        widgets.iter().map(|x| (x.id.clone(), x.clone())).collect();

    Ok(widgets)
}

#[tauri::command]
pub fn get_widget(
    cache_state: AppState<CacheState, '_>,
    widgets_path: PathBuf,
    id: String,
) -> Result<Option<Widget>, Error> {
    let guard = cache_state.widgets_cache.read();

    if !guard.is_empty() {
        return Ok(guard.get(&id).cloned());
    }

    drop(guard);

    let widgets = utils::get_widgets(widgets_path)?;
    let mut map = BTreeMap::new();
    let mut widget = None;

    for it in widgets {
        if it.id == id {
            widget = Some(it.clone());
        }

        map.insert(it.id.clone(), it);
    }

    *cache_state.widgets_cache.write() = map;

    Ok(widget)
}

#[tauri::command]
pub fn add_path_to_fs_scope(
    app_handle: AppHandle,
    path: PathBuf,
    recursive: bool,
) -> Result<(), Error> {
    if !path.exists() {
        return Err(Error::new("path does not exist"));
    }

    app_handle.fs_scope().allow_directory(&path, recursive)?;

    Ok(())
}
