use crate::models::DnsProvider;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_speed_unit")]
    pub speed_unit: String,
    #[serde(default = "default_enabled")]
    pub tray_show_speed: bool,
    #[serde(default = "default_enabled")]
    pub tray_show_dns: bool,
    #[serde(default = "default_enabled")]
    pub tray_show_status: bool,
    #[serde(default = "default_enabled")]
    pub tray_show_latency: bool,
    #[serde(default)]
    pub custom_providers: Vec<DnsProvider>,
}

fn default_language() -> String {
    "en".into()
}

fn default_theme() -> String {
    "system".into()
}

fn default_speed_unit() -> String {
    "kbps".into()
}

fn default_enabled() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: default_theme(),
            speed_unit: default_speed_unit(),
            tray_show_speed: true,
            tray_show_dns: true,
            tray_show_status: true,
            tray_show_latency: true,
            custom_providers: Vec::new(),
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "UseDNS", "UseDNS").map(|dirs| dirs.config_dir().join("settings.json"))
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension("json.tmp")
}

pub fn load() -> Settings {
    let Some(path) = settings_path() else {
        return Settings {
            language: default_language(),
            ..Default::default()
        };
    };
    let backup = backup_path(&path);
    if !path.exists() && backup.exists() {
        let _ = fs::rename(&backup, &path);
    }

    fs::read_to_string(&path)
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
    let temporary = temporary_path(&path);
    let backup = backup_path(&path);

    let mut file = File::create(&temporary)?;
    file.write_all(&data)?;
    file.sync_all()?;
    drop(file);

    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    if path.exists() {
        fs::rename(&path, &backup)?;
    }
    if let Err(error) = fs::rename(&temporary, &path) {
        if backup.exists() {
            let _ = fs::rename(&backup, &path);
        }
        return Err(error);
    }
    if backup.exists() {
        fs::remove_file(backup)?;
    }
    Ok(())
}
