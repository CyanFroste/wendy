use crate::types::Widget;
use anyhow::Result;
use std::fs::File;
use std::path::PathBuf;

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
        RPC_AUDIO_SET_MASTER_VOLUME => Some(&["audio:write"]),
        RPC_AUDIO_TOGGLE_MASTER_MUTE => Some(&["audio:write"]),
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
