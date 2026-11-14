// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod media;
mod network;
mod system;
mod types;
mod utils;

use anyhow::Result;
use audio::AudioState;
use media::MediaState;
use network::HttpClient;
use system::SystemState;
use types::CacheState;

#[tokio::main]
async fn main() -> Result<()> {
    let http_client = HttpClient::new(5, 10 * 1024 * 1024)?; // 10MB
    let cache_state = CacheState::default();
    let media_state = MediaState::default();
    let audio_state = AudioState::new();
    let system_state = SystemState::new()?;

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_wallpaper::init())
        .manage(http_client)
        .manage(cache_state)
        .manage(system_state)
        .manage(media_state)
        .manage(audio_state)
        .on_window_event(utils::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::widget_rpc,
            commands::get_widgets,
            commands::get_widget,
            commands::add_path_to_fs_scope,
            commands::get_widget_asset,
            commands::start_media_listener,
            commands::start_audio_capture,
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}
