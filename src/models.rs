use serde::{Deserialize, Serialize};
use std::{borrow::Cow, net::IpAddr};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsProfile {
    pub id: String,
    pub name_en: String,
    pub name_id: String,
    pub description_en: String,
    pub description_id: String,
    pub tag_en: String,
    pub tag_id: String,
    pub ipv4_primary: String,
    pub ipv4_secondary: String,
    pub ipv6_primary: String,
    pub ipv6_secondary: String,
    /// DNS-over-HTTPS template for this profile. Leave it empty for resolvers
    /// that are reachable over plain DNS only.
    #[serde(default)]
    pub doh_template: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsProvider {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub ipv4_primary: String,
    pub ipv4_secondary: String,
    pub ipv6_primary: String,
    pub ipv6_secondary: String,
    pub summary_en: String,
    pub summary_id: String,
    #[serde(default)]
    pub purpose: String,
    pub pros_en: String,
    pub pros_id: String,
    pub cons_en: String,
    pub cons_id: String,
    pub icon: String,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub profiles: Vec<DnsProfile>,
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
        for profile in &self.profiles {
            validate_pair(&profile.ipv4_primary, &profile.ipv4_secondary, false)?;
            validate_pair(&profile.ipv6_primary, &profile.ipv6_secondary, true)?;
        }
        Ok(())
    }

    pub fn get_profiles(&self) -> Cow<'_, [DnsProfile]> {
        if !self.profiles.is_empty() {
            return Cow::Borrowed(&self.profiles);
        }
        Cow::Owned(vec![DnsProfile {
            id: "default".into(),
            name_en: "Default".into(),
            name_id: "Bawaan".into(),
            description_en: self.summary_en.clone(),
            description_id: self.summary_id.clone(),
            tag_en: "DEFAULT".into(),
            tag_id: "BAWAAN".into(),
            ipv4_primary: self.ipv4_primary.clone(),
            ipv4_secondary: self.ipv4_secondary.clone(),
            ipv6_primary: self.ipv6_primary.clone(),
            ipv6_secondary: self.ipv6_secondary.clone(),
            doh_template: String::new(),
        }])
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

/// Registry that ships with the binary. `dns-register/dns.json` is the single
/// source of truth and is also published over CDN, so this only serves as the
/// last-resort fallback when the CDN and the local file are both unavailable.
pub fn default_providers() -> Vec<DnsProvider> {
    serde_json::from_str(include_str!("../dns-register/dns.json")).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_registry_is_valid() {
        let providers = default_providers();
        assert!(!providers.is_empty(), "bundled registry is empty");
        for provider in providers {
            assert!(provider.validate().is_ok(), "{}", provider.name);
            assert!(
                !provider.get_profiles().is_empty(),
                "Profiles for {}",
                provider.name
            );
        }
    }

    #[test]
    fn rejects_wrong_ip_version() {
        let mut provider = default_providers().remove(0);
        provider.ipv4_primary = "::1".into();
        assert!(provider.validate().is_err());
    }
}
