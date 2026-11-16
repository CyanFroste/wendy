use crate::types::Widget;
use anyhow::Result;
use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::io::Cursor;
use std::path::{Component as PathComponent, Path, PathBuf};
use tokio::fs;
use zip::ZipArchive;

pub mod constants {
    pub const RPC_NETWORK_REQUEST: &str = "network.request";
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
    pub const RPC_SYSTEM_READ_DIR: &str = "system.read_dir";
    pub const RPC_SYSTEM_READ_FILE: &str = "system.read_file";
    pub const RPC_STORE_ENTRIES: &str = "store.entries";
    pub const RPC_STORE_KEYS: &str = "store.keys";
    pub const RPC_STORE_VALUES: &str = "store.values";
    pub const RPC_STORE_GET: &str = "store.get";
    pub const RPC_STORE_SET: &str = "store.set";
    pub const RPC_STORE_DELETE: &str = "store.delete";
    pub const RPC_STORE_CLEAR: &str = "store.clear";
}

pub fn get_rpc_required_scopes(method: &str) -> Option<&'static [&'static str]> {
    use constants::*;

    match method {
        RPC_NETWORK_REQUEST => Some(&["network:exec"]),
        RPC_SYSTEM_GET_CPU_INFO => Some(&["system:read"]),
        RPC_SYSTEM_GET_RAM_INFO => Some(&["system:read"]),
        RPC_SYSTEM_GET_DISK_INFO => Some(&["system:read"]),
        RPC_SYSTEM_GET_INSTALLED_APPS => Some(&["system:read"]),
        RPC_SYSTEM_OPEN_INSTALLED_APP => Some(&["system:exec"]),
        RPC_MEDIA_GET_ACTIVE_MEDIA => Some(&["media:read"]),
        RPC_MEDIA_SEND_COMMAND => Some(&["media:exec"]),
        RPC_AUDIO_GET_MASTER_VOLUME => Some(&["audio:read"]),
        RPC_AUDIO_SET_MASTER_VOLUME => Some(&["audio:exe"]),
        RPC_AUDIO_TOGGLE_MASTER_MUTE => Some(&["audio:exec"]),
        RPC_SYSTEM_OPEN_RECYCLE_BIN => Some(&["system:exec"]),
        RPC_SYSTEM_GET_RECYCLE_BIN_COUNT => Some(&["system:read"]),
        RPC_SYSTEM_READ_DIR => Some(&["system:read"]),
        RPC_SYSTEM_READ_FILE => Some(&["system:read"]),
        RPC_STORE_ENTRIES => Some(&["store:read"]),
        RPC_STORE_KEYS => Some(&["store:read"]),
        RPC_STORE_VALUES => Some(&["store:read"]),
        RPC_STORE_GET => Some(&["store:read"]),
        RPC_STORE_SET => Some(&["store:write"]),
        RPC_STORE_DELETE => Some(&["store:write"]),
        RPC_STORE_CLEAR => Some(&["store:write"]),
        _ => None,
    }
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

pub async fn create_new_project(
    widgets_path: impl AsRef<Path>,
    widget: &Widget,
) -> Result<PathBuf> {
    let widget_path = widgets_path.as_ref().join(&widget.id);
    let content_path = widget_path.join("content");
    let editor_path = widget_path.join("editor");
    let assets_path = widget_path.join("assets");
    let types_path = widget_path.join("types");

    fs::create_dir_all(&widget_path).await?;
    fs::create_dir(&content_path).await?;
    fs::create_dir(&editor_path).await?;
    fs::create_dir(&assets_path).await?;
    fs::create_dir(&types_path).await?;

    let content_jsconfig = include_str!("../templates/content.jsconfig.json");
    let content_css = include_str!("../templates/content.css");
    let editor_jsconfig = include_str!("../templates/editor.jsconfig.json");
    let editor_js = include_str!("../templates/editor.js");
    let common_d_ts = include_str!("../templates/common.d.ts");
    let content_d_ts = include_str!("../templates/content.d.ts");
    let editor_d_ts = include_str!("../templates/editor.d.ts");

    fs::write(&content_path.join("jsconfig.json"), content_jsconfig).await?;
    fs::write(&content_path.join("styles.css"), content_css).await?;
    fs::File::create(&content_path.join("index.html")).await?;
    fs::File::create(&content_path.join("script.js")).await?;

    fs::write(&editor_path.join("jsconfig.json"), editor_jsconfig).await?;
    fs::write(&editor_path.join("script.js"), editor_js).await?;
    fs::File::create(&editor_path.join("index.html")).await?;
    fs::File::create(&editor_path.join("styles.css")).await?;

    fs::write(&types_path.join("common.d.ts"), common_d_ts).await?;
    fs::write(&types_path.join("content.d.ts"), content_d_ts).await?;
    fs::write(&types_path.join("editor.d.ts"), editor_d_ts).await?;

    let manifest = serde_json::to_string_pretty(widget)?;
    let base_jsconfig = include_str!("../templates/base.jsconfig.json");
    let readme = include_str!("../templates/README.md");

    fs::write(&widget_path.join("base.jsconfig.json"), base_jsconfig).await?;
    fs::write(&widget_path.join("manifest.json"), manifest).await?;
    fs::write(&widget_path.join("README.md"), readme).await?;

    Ok(widget_path)
}

pub async fn download_and_install_latest_default_widgets(
    widgets_path: impl AsRef<Path>,
) -> Result<()> {
    let target_dir = widgets_path.as_ref().to_path_buf();

    let bytes =
        reqwest::get("https://github.com/CyanFroste/widgets/releases/latest/download/widgets.zip")
            .await?
            .error_for_status()?
            .bytes()
            .await?;

    tokio::task::spawn_blocking(move || -> Result<()> {
        // snapshot existing top-level folders before extracting anything
        let existing: HashSet<_> = std::fs::read_dir(&target_dir)?
            .filter_map(|entry| entry.ok().map(|e| e.file_name()))
            .collect();

        let mut archive = ZipArchive::new(Cursor::new(bytes))?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;

            let Some(enclosed_path) = entry.enclosed_name() else {
                continue;
            };

            let Some(PathComponent::Normal(top_dir)) = enclosed_path.components().next() else {
                continue;
            };

            if existing.contains(top_dir) {
                continue;
            }

            let out_path = target_dir.join(&enclosed_path);

            if !entry.is_dir() {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut out_file = File::create(&out_path)?;

                io::copy(&mut entry, &mut out_file)?;
            } else {
                std::fs::create_dir_all(&out_path)?;
            }
        }

        Ok(())
    })
    .await??;

    Ok(())
}
