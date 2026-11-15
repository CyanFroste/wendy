use crate::audio::AudioState;
use crate::db::DbState;
use crate::media::{MediaCommand, MediaState};
use crate::network::{HttpClient, HttpOptions};
use crate::system::{InstalledApp, ScopedDir, SystemState};
use crate::types::{CacheState, Error, Widget};
use crate::utils;
use crate::utils::constants;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager, State as AppState};
use url::Url;

#[tauri::command]
pub fn start_audio_capture(
    app_handle: AppHandle,
    audio_state: AppState<AudioState, '_>,
    window_label: String,
) {
    let app_handle = app_handle.clone();

    audio_state.start_capture(window_label, move |chunk| {
        let subscribers: Vec<String> = app_handle
            .state::<AudioState>()
            .subscribers
            .lock()
            .iter()
            .cloned()
            .collect();

        for subscriber in subscribers {
            let _ = app_handle.emit_to(&subscriber, "audio-captured", &chunk);
        }
    });
}

#[tauri::command]
pub async fn start_media_listener(
    app_handle: AppHandle,
    media_state: AppState<MediaState, '_>,
    window_label: String,
) -> Result<(), Error> {
    let rt = tokio::runtime::Handle::current();

    media_state
        .start_listener(window_label, move || {
            let app_handle = app_handle.clone();
            let rt = rt.clone();

            rt.clone().spawn_blocking(move || {
                let media_state = app_handle.state::<MediaState>();
                let subscribers: Vec<String> =
                    media_state.subscribers.lock().iter().cloned().collect();

                if subscribers.is_empty() {
                    return;
                }

                if let Ok(data) = rt.block_on(media_state.get_active_media()) {
                    for subscriber in subscribers {
                        let _ = app_handle.emit_to(&subscriber, "media-updated", &data);
                    }
                }
            });
        })
        .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn widget_rpc(
    app_handle: AppHandle,
    http_client: AppState<HttpClient, '_>,
    cache_state: AppState<CacheState, '_>,
    media_state: AppState<MediaState, '_>,
    audio_state: AppState<AudioState, '_>,
    system_state: AppState<SystemState, '_>,
    db_state: AppState<DbState, '_>,
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
        constants::RPC_NETWORK_REQUEST => {
            let url: Url = params["url"]
                .as_str()
                .ok_or_else(|| Error::new("missing parameter 'url'"))?
                .parse()
                .map_err(Error::from_error)?;
            let options: Option<HttpOptions> = serde_json::from_value(params["options"].clone())?;

            let data = http_client
                .request(&url, options, &widget.allowed_domains)
                .await?;

            Ok(serde_json::to_value(&data)?)
        }

        constants::RPC_AUDIO_GET_MASTER_VOLUME => {
            let data = audio_state.get_master_volume().await?;

            Ok(serde_json::to_value(&data)?)
        }

        constants::RPC_AUDIO_SET_MASTER_VOLUME => {
            let level = params["level"]
                .as_f64()
                .ok_or_else(|| Error::new("missing parameter 'level'"))?;
            let level = level.clamp(0.0, 1.0) as f32;
            let data = audio_state.set_master_volume(level).await?;

            Ok(serde_json::to_value(&data)?)
        }

        constants::RPC_AUDIO_TOGGLE_MASTER_MUTE => {
            let data = audio_state.toggle_master_mute().await?;

            Ok(serde_json::to_value(&data)?)
        }

        constants::RPC_MEDIA_GET_ACTIVE_MEDIA => {
            let rt = tokio::runtime::Handle::current();

            let data = tokio::task::spawn_blocking(move || {
                let app_handle = app_handle.clone();
                let media_state = app_handle.state::<MediaState>();

                rt.block_on(media_state.get_active_media())
            })
            .await
            .map_err(Error::from_error)??;

            Ok(serde_json::to_value(&data)?)
        }

        constants::RPC_MEDIA_SEND_COMMAND => {
            let player_id = params["playerId"]
                .as_str()
                .ok_or_else(|| Error::new("missing parameter 'playerId'"))?
                .to_string();
            let command: MediaCommand = serde_json::from_value(params["command"].clone())?;

            media_state.send_command(player_id, command).await?;

            Ok(JsonValue::Null)
        }

        constants::RPC_SYSTEM_READ_DIR => {
            let path = params["path"]
                .as_str()
                .ok_or_else(|| Error::new("missing parameter 'path'"))?;
            let data = system_state.read_dir(path)?;

            Ok(serde_json::to_value(&data)?)
        }

        constants::RPC_SYSTEM_READ_FILE => {
            let path = params["path"]
                .as_str()
                .ok_or_else(|| Error::new("missing parameter 'path'"))?;
            let data = system_state.read_file(path)?;

            Ok(serde_json::to_value(&data)?)
        }

        constants::RPC_SYSTEM_GET_CPU_INFO => {
            let data = system_state.get_cpu_info();

            Ok(serde_json::to_value(&data)?)
        }

        constants::RPC_SYSTEM_GET_RAM_INFO => {
            let data = system_state.get_ram_info();

            Ok(serde_json::to_value(&data)?)
        }

        constants::RPC_SYSTEM_GET_DISK_INFO => {
            let data = system_state.get_disk_info()?;

            Ok(serde_json::to_value(&data)?)
        }

        constants::RPC_SYSTEM_GET_INSTALLED_APPS => {
            Ok(serde_json::to_value(&system_state.installed_apps)?)
        }

        constants::RPC_SYSTEM_OPEN_INSTALLED_APP => {
            let path = params["path"]
                .as_str()
                .ok_or_else(|| Error::new("missing parameter 'path'"))?;
            let path = PathBuf::from(path);

            if !system_state.installed_apps.iter().any(|x| x.path == path) {
                return Err(Error::new("invalid application path"));
            }

            // ! NOTE: need to make sure this doesn't make the launched app bound to the current process
            open::that_detached(path)?;

            Ok(JsonValue::Null)
        }

        constants::RPC_SYSTEM_GET_RECYCLE_BIN_COUNT => {
            let count = system_state.get_recycle_bin_count().await?;

            Ok(JsonValue::Number(count.into()))
        }

        constants::RPC_SYSTEM_OPEN_RECYCLE_BIN => {
            open::that_detached("shell:RecycleBinFolder")?;

            Ok(JsonValue::Null)
        }

        constants::RPC_STORE_ENTRIES
        | constants::RPC_STORE_KEYS
        | constants::RPC_STORE_VALUES
        | constants::RPC_STORE_GET
        | constants::RPC_STORE_SET
        | constants::RPC_STORE_DELETE
        | constants::RPC_STORE_CLEAR => {
            let name = params["name"]
                .as_str()
                .ok_or_else(|| Error::new("missing parameter 'name'"))?;
            let store = db_state.get_store(name, &widget.allowed_stores)?;

            let key = params["key"]
                .as_str()
                .ok_or_else(|| Error::new("missing parameter 'key'"));

            match method.as_str() {
                constants::RPC_STORE_ENTRIES => {
                    let data = store.entries().await?;

                    Ok(serde_json::to_value(&data)?)
                }

                constants::RPC_STORE_VALUES => {
                    let data = store.values().await?;

                    Ok(serde_json::to_value(&data)?)
                }

                constants::RPC_STORE_KEYS => {
                    let data = store.keys().await?;

                    Ok(serde_json::to_value(&data)?)
                }

                constants::RPC_STORE_GET => {
                    let data = store.get(key?).await?;

                    Ok(serde_json::to_value(&data)?)
                }

                constants::RPC_STORE_SET => {
                    let value = params["value"].clone();

                    store.set(key?, value).await?;

                    Ok(JsonValue::Null)
                }

                constants::RPC_STORE_DELETE => {
                    let data = store.delete(key?).await?;

                    Ok(serde_json::to_value(&data)?)
                }

                constants::RPC_STORE_CLEAR => {
                    store.clear().await?;

                    Ok(JsonValue::Null)
                }

                _ => unreachable!(),
            }
        }

        _ => Err(Error::new("unknown method")),
    }
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
pub async fn read_file(
    system_state: AppState<SystemState, '_>,
    path: String,
) -> Result<Response, Error> {
    let bytes = system_state.read_file(&path)?;

    Ok(Response::new(bytes))
}

#[tauri::command]
pub fn get_fs_scope_list(system_state: AppState<SystemState, '_>) -> Vec<ScopedDir> {
    system_state.get_fs_scope_list()
}

#[tauri::command]
pub fn add_dir_to_fs_scope(
    system_state: AppState<SystemState, '_>,
    path: PathBuf,
    recursive: bool,
) -> Result<(), Error> {
    system_state.add_dir_to_fs_scope(&path, recursive)?;

    Ok(())
}

#[tauri::command]
pub fn remove_dir_from_fs_scope(
    system_state: AppState<SystemState, '_>,
    path: PathBuf,
    recursive: bool,
) -> Result<(), Error> {
    system_state.remove_dir_from_fs_scope(&path, recursive)?;

    Ok(())
}

#[tauri::command]
pub fn get_installed_apps(
    system_state: AppState<SystemState, '_>,
) -> Result<Arc<Vec<InstalledApp>>, Error> {
    Ok(Arc::clone(&system_state.installed_apps))
}

#[tauri::command]
pub fn open_installed_app(
    system_state: AppState<SystemState, '_>,
    path: PathBuf,
) -> Result<(), Error> {
    if !system_state.installed_apps.iter().any(|x| x.path == path) {
        return Err(Error::new("invalid application path"));
    }

    // ! NOTE: need to make sure this doesn't make the launched app bound to the current process
    open::that_detached(path)?;

    Ok(())
}

#[tauri::command]
pub async fn create_new_project(
    widgets_path: PathBuf,
    mut widget: Widget,
) -> Result<Widget, Error> {
    widget.path = utils::create_new_project(&widgets_path, &widget).await?;

    Ok(widget)
}

#[tauri::command]
pub async fn init(db_state: AppState<DbState, '_>) -> Result<(), Error> {
    db_state.init().await?;

    Ok(())
}
