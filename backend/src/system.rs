use crate::types::{ExternalStore, FileEntry};
use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::path::{Component as PathComponent, Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tauri::fs::Scope;
use walkdir::WalkDir;

const FS_SCOPE_LIST_KEY: &str = "scopes";

pub struct SystemState {
    pub system: Arc<Mutex<System>>,
    pub installed_apps: Arc<RwLock<Vec<InstalledApp>>>,
    // TODO: can move this to a ScopeManager, may not use ExternalStore
    pub asset_scope: Scope,
    pub fs_scope: Scope,
    pub fs_scope_store: Arc<ExternalStore>,
}

impl SystemState {
    pub fn new(
        fs_scope: Scope,
        asset_scope: Scope,
        fs_scope_store: Arc<ExternalStore>,
    ) -> Result<Self> {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        )?;

        system.refresh_cpu_all();
        system.refresh_memory();

        Ok(Self {
            system: Arc::new(Mutex::new(system)),
            installed_apps: Arc::new(RwLock::new(Vec::new())),
            asset_scope,
            fs_scope,
            fs_scope_store,
        })
    }

    pub async fn init(&self) -> Result<()> {
        let installed_apps = Arc::clone(&self.installed_apps);
        let system = Arc::clone(&self.system);

        tokio::task::spawn_blocking(move || {
            let mut guard = system.lock();

            guard.refresh_cpu_all();
            guard.refresh_memory();
            drop(guard);

            *installed_apps.write() = get_installed_apps();
        })
        .await?;

        Ok(())
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

    pub fn restore_fs_scope(&self) -> Result<()> {
        let mut list = self.get_fs_scope_list();
        let before = list.len();

        list.retain(|entry| {
            if entry.path.is_dir() {
                self.fs_scope
                    .allow_directory(&entry.path, entry.recursive)
                    .is_ok()
                    && self
                        .asset_scope
                        .allow_directory(&entry.path, entry.recursive)
                        .is_ok()
            } else {
                false
            }
        });

        if list.len() != before {
            self.save_fs_scope_list(&list)?;
        }

        Ok(())
    }

    pub fn validate_fs_scoped_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let raw = path.as_ref();

        // reject relative paths and `..` traversal without stripping drive/root components
        if !raw.is_absolute()
            || raw
                .components()
                .any(|c| matches!(c, PathComponent::ParentDir))
            || !self.fs_scope.is_allowed(raw)
        {
            return Err(anyhow::anyhow!("access denied"));
        }

        // resolve symlinks/junctions and verify the real target is still in scope
        let canonical_path = fs::canonicalize(raw)?;

        if !self.fs_scope.is_allowed(&canonical_path) {
            return Err(anyhow::anyhow!("access denied"));
        }

        if !canonical_path.exists() {
            return Err(anyhow::anyhow!("file not found"));
        }

        Ok(canonical_path)
    }

    pub fn get_fs_scope_list(&self) -> Vec<ScopedDir> {
        self.fs_scope_store
            .get(FS_SCOPE_LIST_KEY)
            .and_then(|x| serde_json::from_value(x).ok())
            .unwrap_or_default()
    }

    pub fn add_dir_to_fs_scope(&self, path: impl AsRef<Path>, recursive: bool) -> Result<()> {
        if !path.as_ref().is_dir() {
            return Err(anyhow::anyhow!(
                "directory does not exist: {}",
                path.as_ref().display()
            ));
        }

        let clean_path = normalize_display_path(path);
        let key = path_as_key(&clean_path);

        self.fs_scope.allow_directory(&clean_path, recursive)?;
        self.asset_scope.allow_directory(&clean_path, recursive)?;

        let mut list = self.get_fs_scope_list();

        if let Some(existing) = list
            .iter_mut()
            .find(|x| path_as_key(normalize_display_path(&x.path)) == key)
        {
            existing.path = clean_path;
            existing.recursive = recursive;
        } else {
            list.push(ScopedDir {
                path: clean_path,
                recursive,
            });
        }

        self.save_fs_scope_list(&list)
    }

    pub fn remove_dir_from_fs_scope(&self, path: impl AsRef<Path>, recursive: bool) -> Result<()> {
        let clean_path = normalize_display_path(path);
        let target_key = path_as_key(&clean_path);

        // Best-effort revoke for the current session (may fail if folder was already deleted)
        let _ = self.fs_scope.forbid_directory(&clean_path, recursive);
        let _ = self.asset_scope.forbid_directory(&clean_path, recursive);

        let mut list = self.get_fs_scope_list();

        list.retain(|entry| {
            let entry_key = path_as_key(normalize_display_path(&entry.path));

            if recursive {
                !entry_key.starts_with(&target_key)
            } else {
                entry_key != target_key
            }
        });

        self.save_fs_scope_list(&list)
    }

    fn save_fs_scope_list(&self, list: &[ScopedDir]) -> Result<()> {
        self.fs_scope_store
            .set(FS_SCOPE_LIST_KEY, serde_json::to_value(list)?);
        self.fs_scope_store.save()?;

        Ok(())
    }

    pub fn read_dir(&self, path: impl AsRef<Path>) -> Result<Vec<FileEntry>> {
        let path = self.validate_fs_scoped_path(&path)?;

        if !path.is_dir() {
            return Err(anyhow::anyhow!("directory not found"));
        }

        let entries = path
            .read_dir()?
            .flatten()
            .filter_map(|x| {
                x.file_type().ok().map(|y| FileEntry {
                    is_dir: y.is_dir(),
                    path: x.path(),
                })
            })
            .collect();

        Ok(entries)
    }

    pub fn read_file(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let path = self.validate_fs_scoped_path(&path)?;

        if !path.is_file() {
            return Err(anyhow::anyhow!("file not found"));
        }

        Ok(fs::read(&path)?)
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

    apps.sort_by_key(|a| a.name.to_lowercase());
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

fn normalize_display_path(path: impl AsRef<Path>) -> PathBuf {
    let resolved = fs::canonicalize(&path).unwrap_or_else(|_| path.as_ref().to_path_buf());
    let lossy = resolved.to_string_lossy();

    let without_prefix = if let Some(unc) = lossy.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        lossy.strip_prefix(r"\\?\").unwrap_or(&lossy).to_owned()
    };

    let trimmed = without_prefix.trim_end_matches(['\\', '/']);

    if trimmed.is_empty() && !without_prefix.is_empty() {
        PathBuf::from("/")
    } else if trimmed.ends_with(':') {
        // Keep trailing slash so "C:\" doesn't become drive-relative "C:"
        PathBuf::from(format!(r"{trimmed}\"))
    } else {
        PathBuf::from(trimmed)
    }
}

fn path_as_key(path: impl AsRef<Path>) -> PathBuf {
    // lowercased path for component-safe, case-insensitive matching on Windows.
    PathBuf::from(path.as_ref().to_string_lossy().to_ascii_lowercase())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedDir {
    pub path: PathBuf,
    pub recursive: bool,
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
