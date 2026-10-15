use crate::models::DnsProvider;
use curl::easy::Easy;
use std::fs;
use std::path::Path;
use std::time::Duration;

pub const CDN_BASE: &str = "https://cdn.jsdelivr.net/gh/x-labs-myid/UseDNS@main/dns-register/";

pub const CDN_OPTIONS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/x-labs-myid/UseDNS@main/dns-register/dns.json",
    "https://cdn.statically.io/gh/x-labs-myid/UseDNS@main/dns-register/dns.json",
    "https://raw.githubusercontent.com/x-labs-myid/UseDNS/refs/heads/main/dns-register/dns.json",
];

pub struct DnsLoader;

impl DnsLoader {
    /// Load DNS providers from CDN with local fallback.
    pub fn load() -> Vec<DnsProvider> {
        // Try each CDN option in order
        for &url in CDN_OPTIONS {
            match Self::fetch_url(url) {
                Ok(data) => {
                    if let Ok(providers) = serde_json::from_slice::<Vec<DnsProvider>>(&data) {
                        return providers;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch from {}: {}", url, e);
                }
            }
        }

        // Fallback to local file
        Self::load_local().unwrap_or_else(|_| crate::models::default_providers())
    }

    /// Load icon data, trying local first then CDN.
    pub fn load_icon(icon_path: &str) -> Option<Vec<u8>> {
        // 1. Try local
        let local_path = Path::new("dns-register").join(icon_path);
        if local_path.exists() {
            if let Ok(data) = fs::read(local_path) {
                return Some(data);
            }
        }

        // 2. Try CDN
        let url = format!("{}{}", CDN_BASE, icon_path.replace("\\", "/"));
        Self::fetch_url(&url).ok()
    }

    fn fetch_url(url: &str) -> Result<Vec<u8>, String> {
        let mut data = Vec::new();
        let mut easy = Easy::new();
        easy.url(url).map_err(|e| e.to_string())?;
        easy.timeout(Duration::from_secs(10))
            .map_err(|e| e.to_string())?;
        easy.follow_location(true).map_err(|e| e.to_string())?;
        easy.fail_on_error(true).map_err(|e| e.to_string())?;
        easy.useragent("UseDNS/1.0").map_err(|e| e.to_string())?;

        {
            let mut transfer = easy.transfer();
            transfer
                .write_function(|new_data| {
                    data.extend_from_slice(new_data);
                    Ok(new_data.len())
                })
                .map_err(|e| e.to_string())?;
            transfer.perform().map_err(|e| e.to_string())?;
        }

        Ok(data)
    }

    fn load_local() -> Result<Vec<DnsProvider>, String> {
        let path = Path::new("dns-register/dns.json");
        if !path.exists() {
            return Err("Local dns.json not found".into());
        }

        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }
}
