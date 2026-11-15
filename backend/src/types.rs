use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct CacheState {
    pub widgets_cache: RwLock<BTreeMap<String, Widget>>,
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
    #[serde(default)]
    pub allowed_stores: Vec<String>,
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
