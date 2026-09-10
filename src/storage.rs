use crate::models::DnsProvider;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

#[derive(Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub custom_providers: Vec<DnsProvider>,
}

fn default_language() -> String {
    "en".into()
}

fn settings_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "UseDNS", "UseDNS").map(|dirs| dirs.config_dir().join("settings.json"))
}

pub fn load() -> Settings {
    let Some(path) = settings_path() else {
        return Settings {
            language: default_language(),
            ..Default::default()
        };
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_else(|| Settings {
            language: default_language(),
            ..Default::default()
        })
}

pub fn save(settings: &Settings) -> io::Result<()> {
    let path = settings_path()
        .ok_or_else(|| io::Error::other("configuration directory is unavailable"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
    fs::write(path, data)
}
