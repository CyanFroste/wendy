use crate::audio::AudioState;
use crate::db::DbState;
use crate::media::MediaCommand;
use crate::media::MediaState;
use crate::network::HttpClient;
use crate::network::HttpOptions;
use crate::system::SystemState;
use crate::types::{CacheState, Error, Widget};
use crate::utils;
use crate::utils::constants;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager, State as AppState};
use tauri_plugin_fs::FsExt;
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

#[tauri::command]
pub async fn init(db_state: AppState<DbState, '_>) -> Result<(), Error> {
    db_state.init().await?;

    Ok(())
}
