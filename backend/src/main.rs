// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod media;
mod types;
mod utils;

use anyhow::Result;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use types::{CacheState, MediaState, SystemState};

#[tokio::main]
async fn main() -> Result<()> {
    let mut system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    )?;

    system.refresh_cpu_all();
    system.refresh_memory();

    let system_state = SystemState::new(system);
    let media_state = MediaState::default();
    let mut cache_state = CacheState::default();

    cache_state.installed_apps = utils::get_installed_apps().await?;

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_wallpaper::init())
        .manage(cache_state)
        .manage(system_state)
        .manage(media_state)
        .on_window_event(utils::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::widget_rpc,
            commands::get_widgets,
            commands::get_widget,
            commands::add_path_to_fs_scope,
            commands::get_widget_asset,
            commands::start_media_listener,
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}
