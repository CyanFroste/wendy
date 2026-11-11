// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod types;
mod utils;

use anyhow::Result;
use types::State;

#[tokio::main]
async fn main() -> Result<()> {
    let state = State::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_wallpaper::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_widgets,
            commands::get_widget,
            commands::add_path_to_fs_scope,
            commands::get_widget_asset
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}
