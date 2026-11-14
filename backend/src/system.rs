use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use std::{env, fs};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use walkdir::WalkDir;

pub struct SystemState {
    pub system: Mutex<System>,
    pub installed_apps: Vec<InstalledApp>,
}

impl SystemState {
    pub fn new() -> Result<Self> {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        )?;
        let installed_apps = get_installed_apps();

        system.refresh_cpu_all();
        system.refresh_memory();

        Ok(Self {
            system: Mutex::new(system),
            installed_apps,
        })
    }

    pub fn get_cpu_info(&self) -> CpuInfo {
        let mut system = self.system.lock();
        system.refresh_cpu_all();
        let cpus = system.cpus();

        CpuInfo {
            brand: cpus
                .first()
                .map(|c| c.brand().to_string())
                .unwrap_or_default(),
            global_usage: system.global_cpu_usage(),
            core_count: cpus.len(),
            core_usages: cpus.iter().map(|c| c.usage()).collect(),
        }
    }

    pub fn get_ram_info(&self) -> RamInfo {
        let mut system = self.system.lock();
        system.refresh_memory();

        RamInfo {
            total_bytes: system.total_memory(),
            used_bytes: system.used_memory(),
            available_bytes: system.available_memory(),
            swap_total_bytes: system.total_swap(),
            swap_used_bytes: system.used_swap(),
        }
    }

    pub fn get_disk_info(&self) -> Result<Vec<DiskInfo>> {
        let data = Disks::new_with_refreshed_list()?
            .iter()
            .map(|disk| DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                file_system: disk.file_system().to_string_lossy().to_string(),
                total_bytes: disk.total_space(),
                available_bytes: disk.available_space(),
                is_removable: disk.is_removable(),
            })
            .collect();

        Ok(data)
    }

    pub async fn get_recycle_bin_count(&self) -> Result<usize> {
        Ok(tokio::task::spawn_blocking(get_recycle_bin_count).await?)
    }
}

fn get_installed_apps() -> Vec<InstalledApp> {
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

                let icon_base64 = get_app_icon(path.to_string_lossy());
                let path = path.to_path_buf();

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

    apps
}

fn get_recycle_bin_count() -> usize {
    let mut count: usize = 0;

    for drive in b'A'..=b'Z' {
        let bin_root = PathBuf::from(format!("{}:\\$Recycle.Bin", drive as char));
        let Ok(sid_dirs) = fs::read_dir(bin_root) else {
            continue;
        };

        for sid in sid_dirs.flatten() {
            let sid_path = sid.path();
            let Ok(entries) = fs::read_dir(&sid_path) else {
                continue;
            };

            count += entries
                .flatten()
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .and_then(|n| n.strip_prefix("$I"))
                        .is_some_and(|suffix| sid_path.join(format!("$R{suffix}")).exists())
                })
                .count();
        }
    }

    count
}

fn get_app_icon(path: impl AsRef<str>) -> Option<String> {
    let path = path.as_ref().to_string();

    systemicons::get_icon(&path, 32)
        .ok()
        .map(|bytes| format!("data:image/png;base64,{}", BASE64.encode(bytes)))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    pub name: String,
    pub path: PathBuf,
    pub icon_base64: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub brand: String,
    pub global_usage: f32,
    pub core_count: usize,
    pub core_usages: Vec<f32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RamInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub is_removable: bool,
}
