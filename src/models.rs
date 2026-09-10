use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsProvider {
    pub id: String,
    pub name: String,
    pub ipv4_primary: String,
    pub ipv4_secondary: String,
    pub ipv6_primary: String,
    pub ipv6_secondary: String,
    pub summary_en: String,
    pub summary_id: String,
    pub pros_en: String,
    pub pros_id: String,
    pub cons_en: String,
    pub cons_id: String,
    #[serde(default)]
    pub custom: bool,
}

impl DnsProvider {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("DNS name cannot be empty".into());
        }
        validate_pair(&self.ipv4_primary, &self.ipv4_secondary, false)?;
        validate_pair(&self.ipv6_primary, &self.ipv6_secondary, true)?;
        if self.ipv4_primary.trim().is_empty() && self.ipv6_primary.trim().is_empty() {
            return Err("At least one primary DNS address is required".into());
        }
        Ok(())
    }
}

fn validate_pair(primary: &str, secondary: &str, ipv6: bool) -> Result<(), String> {
    if primary.trim().is_empty() && !secondary.trim().is_empty() {
        return Err("A secondary address requires a primary address".into());
    }
    for value in [primary, secondary] {
        if value.trim().is_empty() {
            continue;
        }
        let address = value
            .trim()
            .parse::<IpAddr>()
            .map_err(|_| format!("Invalid IP address: {value}"))?;
        if address.is_ipv6() != ipv6 {
            return Err(format!("Address has the wrong IP version: {value}"));
        }
    }
    Ok(())
}

pub fn default_providers() -> Vec<DnsProvider> {
    vec![
        DnsProvider {
            id: "cloudflare".into(),
            name: "Cloudflare".into(),
            ipv4_primary: "1.1.1.1".into(),
            ipv4_secondary: "1.0.0.1".into(),
            ipv6_primary: "2606:4700:4700::1111".into(),
            ipv6_secondary: "2606:4700:4700::1001".into(),
            summary_en: "Fast public resolver with a strong privacy focus.".into(),
            summary_id: "Resolver publik cepat yang berfokus pada privasi.".into(),
            pros_en: "Fast, privacy-minded, broad global network".into(),
            pros_id: "Cepat, mengutamakan privasi, jaringan global luas".into(),
            cons_en: "Does not block ads or adult content by default".into(),
            cons_id: "Tidak memblokir iklan atau konten dewasa secara bawaan".into(),
            custom: false,
        },
        DnsProvider {
            id: "google".into(),
            name: "Google Public DNS".into(),
            ipv4_primary: "8.8.8.8".into(),
            ipv4_secondary: "8.8.4.4".into(),
            ipv6_primary: "2001:4860:4860::8888".into(),
            ipv6_secondary: "2001:4860:4860::8844".into(),
            summary_en: "Reliable resolver backed by Google's global infrastructure.".into(),
            summary_id: "Resolver andal yang didukung infrastruktur global Google.".into(),
            pros_en: "High availability, reliable, widely supported".into(),
            pros_id: "Ketersediaan tinggi, andal, didukung secara luas".into(),
            cons_en: "Some users may prefer a provider outside Google's ecosystem".into(),
            cons_id: "Sebagian pengguna mungkin memilih penyedia di luar ekosistem Google".into(),
            custom: false,
        },
        DnsProvider {
            id: "quad9".into(),
            name: "Quad9".into(),
            ipv4_primary: "9.9.9.9".into(),
            ipv4_secondary: "149.112.112.112".into(),
            ipv6_primary: "2620:fe::fe".into(),
            ipv6_secondary: "2620:fe::9".into(),
            summary_en: "Security-focused resolver that blocks known malicious domains.".into(),
            summary_id: "Resolver keamanan yang memblokir domain berbahaya yang diketahui.".into(),
            pros_en: "Malware and phishing protection, non-profit operator".into(),
            pros_id: "Perlindungan malware dan phishing, dikelola organisasi nirlaba".into(),
            cons_en: "Threat filtering can occasionally block a wanted domain".into(),
            cons_id: "Penyaringan ancaman terkadang dapat memblokir domain yang diinginkan".into(),
            custom: false,
        },
        DnsProvider {
            id: "adguard".into(),
            name: "AdGuard DNS".into(),
            ipv4_primary: "94.140.14.14".into(),
            ipv4_secondary: "94.140.15.15".into(),
            ipv6_primary: "2a10:50c0::ad1:ff".into(),
            ipv6_secondary: "2a10:50c0::ad2:ff".into(),
            summary_en: "Resolver that blocks ads, trackers, and malicious domains.".into(),
            summary_id: "Resolver yang memblokir iklan, pelacak, dan domain berbahaya.".into(),
            pros_en: "Network-wide ad and tracker blocking".into(),
            pros_id: "Pemblokiran iklan dan pelacak untuk seluruh jaringan".into(),
            cons_en: "Filtering can affect sites that depend on ad or tracking domains".into(),
            cons_id: "Penyaringan dapat memengaruhi situs yang bergantung pada domain iklan".into(),
            custom: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_providers_are_valid() {
        for provider in default_providers() {
            assert!(provider.validate().is_ok(), "{}", provider.name);
        }
    }

    #[test]
    fn rejects_wrong_ip_version() {
        let mut provider = default_providers().remove(0);
        provider.ipv4_primary = "::1".into();
        assert!(provider.validate().is_err());
    }
}
