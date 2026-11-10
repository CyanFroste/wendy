use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Default, Debug, Clone)]
pub struct State {
    pub widgets_cache: Arc<RwLock<BTreeMap<String, Widget>>>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
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
    pub permissions: Vec<String>,
    #[serde(skip_deserializing)]
    pub path: PathBuf,
    // pub category: WidgetCategory,
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

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::new(value.to_string())
    }
}
