use crate::types::{InstalledApp, Widget};
use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::env;
use std::fs::File;
use std::path::PathBuf;
use walkdir::WalkDir;

pub const RPC_SYSTEM_GET_INSTALLED_APPS: &str = "system.get_installed_apps";
pub const RPC_SYSTEM_GET_CPU_INFO: &str = "system.get_cpu_info";
pub const RPC_SYSTEM_GET_RAM_INFO: &str = "system.get_ram_info";
pub const RPC_SYSTEM_GET_DISK_INFO: &str = "system.get_disk_info";

pub fn get_rpc_required_scopes(method: &str) -> Option<&'static [&'static str]> {
    match method {
        RPC_SYSTEM_GET_INSTALLED_APPS => Some(&["system:read"]),
        RPC_SYSTEM_GET_CPU_INFO => Some(&["system:read"]),
        RPC_SYSTEM_GET_RAM_INFO => Some(&["system:read"]),
        RPC_SYSTEM_GET_DISK_INFO => Some(&["system:read"]),
        _ => None,
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

                let path = path.to_string_lossy().to_string();
                let icon_base64 = get_app_icon(&path).await?;

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
