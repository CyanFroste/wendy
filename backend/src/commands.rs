use crate::types::{Error, State, Widget};
use crate::utils;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri::AppHandle;
use tauri::State as AppState;
use tauri::ipc::Response;
use tauri_plugin_fs::FsExt;

#[tauri::command]
pub async fn get_widget_asset(
    state: AppState<State, '_>,
    widget_id: String,
    path: String,
) -> Result<Response, Error> {
    // only getting for widgets from cache for now
    let Some(widget_path) = state
        .widgets_cache
        .read()
        .ok()
        .and_then(|x| x.get(&widget_id).map(|x| x.path.clone()))
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
    state: AppState<State, '_>,
    widgets_path: PathBuf,
    refetch: bool,
) -> Result<Vec<Widget>, Error> {
    if let Ok(lock) = state.widgets_cache.read()
        && !refetch
        && !lock.is_empty()
    {
        return Ok(lock.values().cloned().collect());
    }

    let widgets = utils::get_widgets(widgets_path)?;

    if let Ok(mut lock) = state.widgets_cache.write() {
        *lock = widgets.iter().map(|x| (x.id.clone(), x.clone())).collect();
    }

    Ok(widgets)
}

#[tauri::command]
pub fn get_widget(
    state: AppState<State, '_>,
    widgets_path: PathBuf,
    id: String,
) -> Result<Option<Widget>, Error> {
    if let Ok(lock) = state.widgets_cache.read()
        && !lock.is_empty()
    {
        return Ok(lock.get(&id).cloned());
    }

    let widgets = utils::get_widgets(widgets_path)?;
    let mut map = BTreeMap::new();
    let mut widget = None;

    for it in widgets {
        if it.id == id {
            widget = Some(it.clone());
        }

        map.insert(it.id.clone(), it);
    }

    if let Ok(mut lock) = state.widgets_cache.write() {
        *lock = map;
    }

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
