use crate::media::MediaListener;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use sysinfo::System;

#[derive(Default)]
pub struct CacheState {
    pub widgets_cache: RwLock<BTreeMap<String, Widget>>,
    pub installed_apps: Vec<InstalledApp>,
}

pub struct SystemState {
    pub system: Mutex<System>,
}

#[derive(Default)]
pub struct MediaState {
    pub listener: Mutex<Option<MediaListener>>,
    pub subscribers: Mutex<HashSet<String>>,
}

impl SystemState {
    pub fn new(system: System) -> Self {
        Self {
            system: Mutex::new(system),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub preview_path: Option<PathBuf>,
    #[serde(default)]
    pub is_background: bool,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(skip_deserializing)]
    pub path: PathBuf,
    pub author: String,
    pub source: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    pub name: String,
    pub path: PathBuf,
    pub icon_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn from_error(err: impl std::error::Error) -> Self {
        Self::new(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::from_error(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::from_error(value)
    }
}

impl From<tauri::Error> for Error {
    fn from(value: tauri::Error) -> Self {
        Self::from_error(value)
    }
}

impl From<sysinfo::Error> for Error {
    fn from(value: sysinfo::Error) -> Self {
        Self::from_error(value)
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::new(value.to_string())
    }
}
