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
    #[serde(default = "default_network_interval")]
    pub network_interval_secs: u32,
    #[serde(default = "default_enabled")]
    pub tray_show_speed: bool,
    #[serde(default = "default_enabled")]
    pub tray_show_dns: bool,
    #[serde(default = "default_enabled")]
    pub tray_show_status: bool,
    #[serde(default = "default_enabled")]
    pub tray_show_latency: bool,
    #[serde(default)]
    pub start_with_windows: bool,
    #[serde(default)]
    pub custom_providers: Vec<DnsProvider>,
}

fn default_language() -> String {
    "en".into()
}

fn default_theme() -> String {
    "system".into()
}

fn default_network_interval() -> u32 {
    1
}

fn default_enabled() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: default_theme(),
            network_interval_secs: default_network_interval(),
            tray_show_speed: true,
            tray_show_dns: true,
            tray_show_status: true,
            tray_show_latency: true,
            start_with_windows: false,
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
    settings_path()
        .and_then(|path| load_from_path(&path))
        .unwrap_or_default()
}

fn load_from_path(path: &Path) -> Option<Settings> {
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .or_else(|| {
            let backup = backup_path(path);
            let data = fs::read_to_string(&backup).ok()?;
            let settings = serde_json::from_str(&data).ok()?;
            // Recover from an interrupted write or a corrupt main file.
            let _ = fs::copy(&backup, path);
            Some(settings)
        })
}

pub fn save(settings: &Settings) -> io::Result<()> {
    let path = settings_path()
        .ok_or_else(|| io::Error::other("configuration directory is unavailable"))?;
    save_to_path(settings, &path)
}

fn save_to_path(settings: &Settings, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
    let temporary = temporary_path(path);
    let backup = backup_path(path);

    let mut file = File::create(&temporary)?;
    file.write_all(&data)?;
    file.sync_all()?;
    drop(file);

    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    if path.exists() {
        fs::rename(path, &backup)?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(error);
    }
    // Keep the last good settings for recovery; commit has already succeeded.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_default_startup_to_disabled() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(!settings.start_with_windows);
        assert_eq!(settings.network_interval_secs, 1);
        let legacy: Settings = serde_json::from_str(
            r#"{"language":"id","speed_unit":"kbps","network_interval_secs":2}"#,
        )
        .unwrap();
        assert_eq!(legacy.language, "id");
        assert_eq!(legacy.network_interval_secs, 2);
        assert!(
            !serde_json::to_string(&legacy)
                .unwrap()
                .contains("speed_unit")
        );
    }

    #[test]
    fn recovers_settings_after_corruption_or_interrupted_write() {
        let directory = std::env::temp_dir().join(format!(
            "usedns-storage-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = directory.join("settings.json");
        let first = Settings {
            language: "id".into(),
            start_with_windows: true,
            ..Settings::default()
        };
        save_to_path(&first, &path).unwrap();
        save_to_path(&Settings::default(), &path).unwrap();
        fs::write(&path, "{broken").unwrap();
        let recovered = load_from_path(&path).unwrap();
        assert_eq!(recovered.language, "id");
        assert!(recovered.start_with_windows);
        fs::remove_file(&path).unwrap();
        assert!(load_from_path(&path).unwrap().start_with_windows);
        fs::remove_dir_all(directory).unwrap();
    }
}
