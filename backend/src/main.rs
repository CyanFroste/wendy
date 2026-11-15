// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod db;
mod media;
mod network;
mod system;
mod types;
mod utils;

use anyhow::Result;
use audio::AudioState;
use db::Db;
use db::DbState;
use media::MediaState;
use network::HttpClient;
use std::fs;
use system::SystemState;
use tauri::{Manager, WindowEvent};
use tauri_plugin_fs::FsExt;
use tauri_plugin_store::StoreExt;
use types::CacheState;

#[tokio::main]
async fn main() -> Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_wallpaper::init())
        .setup(|app| {
            let data_path = app.path().app_data_dir()?;
            let _ = fs::create_dir_all(&data_path);

            let db = Db::new(data_path.join("wendy.db"));
            let db_state = DbState::new(db);

            let fs_scope = app.fs_scope().clone();
            let asset_scope = app.asset_protocol_scope().clone();
            let fs_scope_store = app.store("fs-scopes-store.json")?;
            let system_state = SystemState::new(fs_scope, asset_scope, fs_scope_store)?;

            let http_client = HttpClient::new(5, 10 * 1024 * 1024)?; // 10MB
            let cache_state = CacheState::default();
            let media_state = MediaState::default();
            let audio_state = AudioState::new();

            system_state.restore_fs_scope()?;

            app.manage(db_state);
            app.manage(system_state);
            app.manage(http_client);
            app.manage(cache_state);
            app.manage(media_state);
            app.manage(audio_state);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Destroyed = event {
                window
                    .state::<MediaState>()
                    .remove_subscriber(window.label());
                window
                    .state::<AudioState>()
                    .remove_subscriber(window.label());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::widget_rpc,
            commands::get_widgets,
            commands::get_widget,
            commands::start_media_listener,
            commands::start_audio_capture,
            commands::get_fs_scope_list,
            commands::add_dir_to_fs_scope,
            commands::remove_dir_from_fs_scope,
            commands::read_file,
            commands::init,
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}
