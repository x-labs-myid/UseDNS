use serde::{Deserialize, Serialize};
use std::net::IpAddr;

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
}

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
    #[serde(default)]
    pub purpose: String,
    pub pros_en: String,
    pub pros_id: String,
    pub cons_en: String,
    pub cons_id: String,
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

    pub fn get_profiles(&self) -> Vec<DnsProfile> {
        if !self.profiles.is_empty() {
            return self.profiles.clone();
        }
        vec![DnsProfile {
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
        }]
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
            purpose: "privacy".into(),
            pros_en: "Fast, privacy-minded, broad global network".into(),
            pros_id: "Cepat, mengutamakan privasi, jaringan global luas".into(),
            cons_en: "Does not block ads or adult content by default".into(),
            cons_id: "Tidak memblokir iklan atau konten dewasa secara bawaan".into(),
            custom: false,
            profiles: vec![
                DnsProfile {
                    id: "standard".into(),
                    name_en: "Standard (Fast & Private)".into(),
                    name_id: "Standar (Cepat & Privasi)".into(),
                    description_en: "Cloudflare standard public DNS without content filtering.".into(),
                    description_id: "DNS publik standar Cloudflare tanpa pemfilteran konten.".into(),
                    tag_en: "STANDARD".into(),
                    tag_id: "STANDAR".into(),
                    ipv4_primary: "1.1.1.1".into(),
                    ipv4_secondary: "1.0.0.1".into(),
                    ipv6_primary: "2606:4700:4700::1111".into(),
                    ipv6_secondary: "2606:4700:4700::1001".into(),
                },
                DnsProfile {
                    id: "security".into(),
                    name_en: "Malware Blocking".into(),
                    name_id: "Blokir Malware & Ancaman".into(),
                    description_en: "Automatically blocks malware, phishing, and known malicious domains.".into(),
                    description_id: "Memblokir malware, phishing, dan domain berbahaya secara otomatis.".into(),
                    tag_en: "SECURITY".into(),
                    tag_id: "KEAMANAN".into(),
                    ipv4_primary: "1.1.1.2".into(),
                    ipv4_secondary: "1.0.0.2".into(),
                    ipv6_primary: "2606:4700:4700::1112".into(),
                    ipv6_secondary: "2606:4700:4700::1002".into(),
                },
                DnsProfile {
                    id: "family".into(),
                    name_en: "Family / Adult Blocking".into(),
                    name_id: "Keluarga (Malware + Konten Dewasa)".into(),
                    description_en: "Blocks malware as well as adult and sexually explicit content.".into(),
                    description_id: "Memblokir malware serta konten dewasa dan pornografi untuk keluarga.".into(),
                    tag_en: "FAMILY SAFE".into(),
                    tag_id: "RAMAH KELUARGA".into(),
                    ipv4_primary: "1.1.1.3".into(),
                    ipv4_secondary: "1.0.0.3".into(),
                    ipv6_primary: "2606:4700:4700::1113".into(),
                    ipv6_secondary: "2606:4700:4700::1003".into(),
                },
            ],
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
            purpose: "general".into(),
            pros_en: "High availability, reliable, widely supported".into(),
            pros_id: "Ketersediaan tinggi, andal, didukung secara luas".into(),
            cons_en: "Some users may prefer a provider outside Google's ecosystem".into(),
            cons_id: "Sebagian pengguna mungkin memilih penyedia di luar ekosistem Google".into(),
            custom: false,
            profiles: vec![
                DnsProfile {
                    id: "standard".into(),
                    name_en: "Standard".into(),
                    name_id: "Standar Global".into(),
                    description_en: "Google's global Anycast DNS network with high availability and low latency.".into(),
                    description_id: "Jaringan DNS Anycast global Google dengan ketersediaan tinggi dan latensi rendah.".into(),
                    tag_en: "SPEED & RELIABLE".into(),
                    tag_id: "KECEPATAN & ANDAL".into(),
                    ipv4_primary: "8.8.8.8".into(),
                    ipv4_secondary: "8.8.4.4".into(),
                    ipv6_primary: "2001:4860:4860::8888".into(),
                    ipv6_secondary: "2001:4860:4860::8844".into(),
                },
            ],
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
            purpose: "security".into(),
            pros_en: "Malware and phishing protection, non-profit operator".into(),
            pros_id: "Perlindungan malware dan phishing, dikelola organisasi nirlaba".into(),
            cons_en: "Threat filtering can occasionally block a wanted domain".into(),
            cons_id: "Penyaringan ancaman terkadang dapat memblokir domain yang diinginkan".into(),
            custom: false,
            profiles: vec![
                DnsProfile {
                    id: "standard".into(),
                    name_en: "Standard (Malware Blocking)".into(),
                    name_id: "Standar (Proteksi Ancaman)".into(),
                    description_en: "Blocks malware, phishing, and spyware with DNSSEC validation enabled.".into(),
                    description_id: "Memblokir malware, phishing, dan spyware dengan validasi DNSSEC aktif.".into(),
                    tag_en: "RECOMMENDED".into(),
                    tag_id: "REKOMENDASI".into(),
                    ipv4_primary: "9.9.9.9".into(),
                    ipv4_secondary: "149.112.112.112".into(),
                    ipv6_primary: "2620:fe::fe".into(),
                    ipv6_secondary: "2620:fe::9".into(),
                },
                DnsProfile {
                    id: "ecs".into(),
                    name_en: "EDNS Client Subnet (ECS)".into(),
                    name_id: "Dengan ECS (Optimalisasi CDN)".into(),
                    description_en: "Includes subnet info to route traffic to geographically closest CDN edges.".into(),
                    description_id: "Menyertakan info subnet untuk merutekan lalu lintas ke server CDN terdekat.".into(),
                    tag_en: "CDN OPTIMIZED".into(),
                    tag_id: "OPTIMAL CDN".into(),
                    ipv4_primary: "9.9.9.11".into(),
                    ipv4_secondary: "149.112.112.11".into(),
                    ipv6_primary: "2620:fe::11".into(),
                    ipv6_secondary: "2620:fe::fe:11".into(),
                },
                DnsProfile {
                    id: "unsecured".into(),
                    name_en: "Unsecured (No Blocking)".into(),
                    name_id: "Tanpa Filter Ancaman".into(),
                    description_en: "Raw resolution without threat blocking or domain filtering.".into(),
                    description_id: "Resolusi langsung tanpa pemblokiran domain ancaman atau filter.".into(),
                    tag_en: "UNFILTERED".into(),
                    tag_id: "TANPA FILTER".into(),
                    ipv4_primary: "9.9.9.10".into(),
                    ipv4_secondary: "149.112.112.10".into(),
                    ipv6_primary: "2620:fe::10".into(),
                    ipv6_secondary: "2620:fe::fe:10".into(),
                },
            ],
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
            purpose: "ad-blocking".into(),
            pros_en: "Network-wide ad and tracker blocking".into(),
            pros_id: "Pemblokiran iklan dan pelacak untuk seluruh jaringan".into(),
            cons_en: "Filtering can affect sites that depend on ad or tracking domains".into(),
            cons_id: "Penyaringan dapat memengaruhi situs yang bergantung pada domain iklan".into(),
            custom: false,
            profiles: vec![
                DnsProfile {
                    id: "default".into(),
                    name_en: "Default (Ad & Tracker Block)".into(),
                    name_id: "Standar (Blokir Iklan & Pelacak)".into(),
                    description_en: "Blocks advertisements, tracking domains, and malicious websites.".into(),
                    description_id: "Memblokir iklan banner, domain pelacak analitik, dan situs berbahaya.".into(),
                    tag_en: "AD BLOCKING".into(),
                    tag_id: "BEBAS IKLAN".into(),
                    ipv4_primary: "94.140.14.14".into(),
                    ipv4_secondary: "94.140.15.15".into(),
                    ipv6_primary: "2a10:50c0::ad1:ff".into(),
                    ipv6_secondary: "2a10:50c0::ad2:ff".into(),
                },
                DnsProfile {
                    id: "family".into(),
                    name_en: "Family Protection".into(),
                    name_id: "Perlindungan Keluarga".into(),
                    description_en: "Blocks ads, trackers, adult content, and enforces SafeSearch.".into(),
                    description_id: "Memblokir iklan, pelacak, konten dewasa, serta mengaktifkan SafeSearch.".into(),
                    tag_en: "FAMILY SAFE".into(),
                    tag_id: "RAMAH KELUARGA".into(),
                    ipv4_primary: "94.140.14.15".into(),
                    ipv4_secondary: "94.140.15.16".into(),
                    ipv6_primary: "2a10:50c0::bad1:ff".into(),
                    ipv6_secondary: "2a10:50c0::bad2:ff".into(),
                },
                DnsProfile {
                    id: "non-filtering".into(),
                    name_en: "Non-Filtering".into(),
                    name_id: "Tanpa Penyaringan".into(),
                    description_en: "Fast, reliable DNS without any ad, tracking, or content filtering.".into(),
                    description_id: "DNS netral dan cepat tanpa pemblokiran iklan atau pelacak apa pun.".into(),
                    tag_en: "UNFILTERED".into(),
                    tag_id: "TANPA FILTER".into(),
                    ipv4_primary: "94.140.14.140".into(),
                    ipv4_secondary: "94.140.14.141".into(),
                    ipv6_primary: "2a10:50c0::1:ff".into(),
                    ipv6_secondary: "2a10:50c0::2:ff".into(),
                },
            ],
        },
        DnsProvider {
            id: "cleanbrowsing".into(),
            name: "CleanBrowsing".into(),
            ipv4_primary: "185.228.168.168".into(),
            ipv4_secondary: "185.228.169.168".into(),
            ipv6_primary: "2a0d:2a00:1::".into(),
            ipv6_secondary: "2a0d:2a00:2::".into(),
            summary_en: "Family-friendly resolver blocking adult content and phishing.".into(),
            summary_id: "Resolver ramah keluarga yang memblokir konten dewasa dan phishing.".into(),
            purpose: "security".into(),
            pros_en: "Safe for kids and families, blocks malicious content".into(),
            pros_id: "Aman untuk keluarga dan anak, memblokir konten berbahaya".into(),
            cons_en: "May restrict access to mixed-content or false-positive domains".into(),
            cons_id: "Dapat membatasi akses ke domain berkonten campuran".into(),
            custom: false,
            profiles: vec![
                DnsProfile {
                    id: "family".into(),
                    name_en: "Family Filter".into(),
                    name_id: "Filter Keluarga".into(),
                    description_en: "Blocks adult content, proxy/VPN bypass sites, and enforces SafeSearch.".into(),
                    description_id: "Memblokir konten dewasa, situs bypass VPN, dan menegakkan SafeSearch.".into(),
                    tag_en: "FAMILY SAFE".into(),
                    tag_id: "RAMAH KELUARGA".into(),
                    ipv4_primary: "185.228.168.168".into(),
                    ipv4_secondary: "185.228.169.168".into(),
                    ipv6_primary: "2a0d:2a00:1::".into(),
                    ipv6_secondary: "2a0d:2a00:2::".into(),
                },
                DnsProfile {
                    id: "adult".into(),
                    name_en: "Adult Filter".into(),
                    name_id: "Filter Dewasa".into(),
                    description_en: "Blocks explicit adult sites while leaving other mixed-content open.".into(),
                    description_id: "Memblokir situs dewasa eksplisit tanpa membatasi situs umum lainnya.".into(),
                    tag_en: "ADULT BLOCK".into(),
                    tag_id: "BLOKIR DEWASA".into(),
                    ipv4_primary: "185.228.168.10".into(),
                    ipv4_secondary: "185.228.169.11".into(),
                    ipv6_primary: "2a0d:2a00:1::1".into(),
                    ipv6_secondary: "2a0d:2a00:2::1".into(),
                },
                DnsProfile {
                    id: "security".into(),
                    name_en: "Security Filter".into(),
                    name_id: "Filter Keamanan".into(),
                    description_en: "Blocks phishing, malware, and malicious domains without filtering adult content.".into(),
                    description_id: "Memblokir phishing, malware, dan domain berbahaya tanpa filter konten dewasa.".into(),
                    tag_en: "SECURITY".into(),
                    tag_id: "KEAMANAN".into(),
                    ipv4_primary: "185.228.168.9".into(),
                    ipv4_secondary: "185.228.169.9".into(),
                    ipv6_primary: "2a0d:2a00:1::2".into(),
                    ipv6_secondary: "2a0d:2a00:2::2".into(),
                },
            ],
        },
        DnsProvider {
            id: "control-d".into(),
            name: "Control D".into(),
            ipv4_primary: "76.76.2.2".into(),
            ipv4_secondary: "76.76.10.2".into(),
            ipv6_primary: "2606:1a40::2".into(),
            ipv6_secondary: "2606:1a40:1::2".into(),
            summary_en: "Modern customizable resolver blocking ads, malware, and trackers.".into(),
            summary_id: "Resolver modern yang memblokir iklan, malware, dan pelacak.".into(),
            purpose: "ad-blocking".into(),
            pros_en: "Fast Anycast network, built-in malware & ad blocking".into(),
            pros_id: "Jaringan Anycast cepat, pemblokir malware & iklan bawaan".into(),
            cons_en: "Custom profile features require an online Control D account".into(),
            cons_id: "Fitur kustomisasi profil lanjutan membutuhkan akun Control D".into(),
            custom: false,
            profiles: vec![
                DnsProfile {
                    id: "ads-malware".into(),
                    name_en: "Ads & Malware Blocking".into(),
                    name_id: "Blokir Iklan, Pelacak & Malware".into(),
                    description_en: "Blocks advertisements, tracking domains, malware, and phishing sites.".into(),
                    description_id: "Memblokir iklan, pelacak data, malware, dan situs penipuan phishing.".into(),
                    tag_en: "ADS & MALWARE".into(),
                    tag_id: "IKLAN & MALWARE".into(),
                    ipv4_primary: "76.76.2.2".into(),
                    ipv4_secondary: "76.76.10.2".into(),
                    ipv6_primary: "2606:1a40::2".into(),
                    ipv6_secondary: "2606:1a40:1::2".into(),
                },
                DnsProfile {
                    id: "malware".into(),
                    name_en: "Malware Protection Only".into(),
                    name_id: "Proteksi Malware Saja".into(),
                    description_en: "Blocks malicious domains and phishing without altering web advertisements.".into(),
                    description_id: "Hanya memblokir malware dan phishing tanpa mengubah iklan pada situs.".into(),
                    tag_en: "SECURITY".into(),
                    tag_id: "KEAMANAN".into(),
                    ipv4_primary: "76.76.2.1".into(),
                    ipv4_secondary: "76.76.10.1".into(),
                    ipv6_primary: "2606:1a40::1".into(),
                    ipv6_secondary: "2606:1a40:1::1".into(),
                },
                DnsProfile {
                    id: "family".into(),
                    name_en: "Family Friendly".into(),
                    name_id: "Ramah Keluarga".into(),
                    description_en: "Blocks adult content, malware, and deceptive websites for household browsing.".into(),
                    description_id: "Memblokir pornografi, malware, dan situs berbahaya untuk seluruh anggota keluarga.".into(),
                    tag_en: "FAMILY SAFE".into(),
                    tag_id: "RAMAH KELUARGA".into(),
                    ipv4_primary: "76.76.2.3".into(),
                    ipv4_secondary: "76.76.10.3".into(),
                    ipv6_primary: "2606:1a40::3".into(),
                    ipv6_secondary: "2606:1a40:1::3".into(),
                },
                DnsProfile {
                    id: "social".into(),
                    name_en: "Block Social Media".into(),
                    name_id: "Blokir Media Sosial".into(),
                    description_en: "Blocks popular social media platforms for productivity and focused study.".into(),
                    description_id: "Memblokir platform media sosial untuk fokus bekerja dan belajar.".into(),
                    tag_en: "PRODUCTIVITY".into(),
                    tag_id: "PRODUKTIVITAS".into(),
                    ipv4_primary: "76.76.2.4".into(),
                    ipv4_secondary: "76.76.10.4".into(),
                    ipv6_primary: "2606:1a40::4".into(),
                    ipv6_secondary: "2606:1a40:1::4".into(),
                },
                DnsProfile {
                    id: "unfiltered".into(),
                    name_en: "Unfiltered / Standard".into(),
                    name_id: "Tanpa Filter / Standar".into(),
                    description_en: "High performance Anycast DNS resolution without any content filters.".into(),
                    description_id: "Resolusi DNS Anycast cepat dan andal tanpa pemfilteran konten apa pun.".into(),
                    tag_en: "UNFILTERED".into(),
                    tag_id: "TANPA FILTER".into(),
                    ipv4_primary: "76.76.2.0".into(),
                    ipv4_secondary: "76.76.10.0".into(),
                    ipv6_primary: "2606:1a40::".into(),
                    ipv6_secondary: "2606:1a40:1::".into(),
                },
            ],
        },
        DnsProvider {
            id: "nextdns".into(),
            name: "NextDNS".into(),
            ipv4_primary: "45.90.28.0".into(),
            ipv4_secondary: "45.90.30.0".into(),
            ipv6_primary: "2a07:a8c0::".into(),
            ipv6_secondary: "2a07:a8c1::".into(),
            summary_en: "Cloud-based resolver with advanced privacy controls and threat prevention.".into(),
            summary_id: "Resolver berbasis cloud dengan privasi tinggi dan pencegahan ancaman.".into(),
            purpose: "privacy".into(),
            pros_en: "Powerful analytics, granular privacy and security filters".into(),
            pros_id: "Analitik andal, filter privasi dan keamanan terperinci".into(),
            cons_en: "Custom blocking rules require linking an account profile ID".into(),
            cons_id: "Aturan pemblokiran kustom memerlukan penautan profil akun".into(),
            custom: false,
            profiles: vec![
                DnsProfile {
                    id: "standard".into(),
                    name_en: "Standard Anycast".into(),
                    name_id: "Standar Anycast".into(),
                    description_en: "Standard Anycast resolver without custom profile linking.".into(),
                    description_id: "Resolver Anycast standar NextDNS tanpa penautan profil akun kustom.".into(),
                    tag_en: "CLOUD PRIVACY".into(),
                    tag_id: "PRIVASI CLOUD".into(),
                    ipv4_primary: "45.90.28.0".into(),
                    ipv4_secondary: "45.90.30.0".into(),
                    ipv6_primary: "2a07:a8c0::".into(),
                    ipv6_secondary: "2a07:a8c1::".into(),
                },
            ],
        },
        DnsProvider {
            id: "opendns".into(),
            name: "OpenDNS".into(),
            ipv4_primary: "208.67.222.222".into(),
            ipv4_secondary: "208.67.220.220".into(),
            ipv6_primary: "2620:119:35::35".into(),
            ipv6_secondary: "2620:119:53::53".into(),
            summary_en: "Reliable enterprise-grade resolver by Cisco with phishing protection.".into(),
            summary_id: "Resolver andal kelas enterprise dari Cisco dengan perlindungan phishing.".into(),
            purpose: "security".into(),
            pros_en: "Backed by Cisco infrastructure, 100% uptime, fast lookup".into(),
            pros_id: "Didukung infrastruktur Cisco, uptime 100%, lookup cepat".into(),
            cons_en: "May redirect NXDOMAIN errors to search suggestions page".into(),
            cons_id: "Dapat mengarahkan error NXDOMAIN ke halaman saran pencarian".into(),
            custom: false,
            profiles: vec![
                DnsProfile {
                    id: "standard".into(),
                    name_en: "Home Standard".into(),
                    name_id: "Standar (Anti-Phishing)".into(),
                    description_en: "Cisco OpenDNS with automatic phishing protection and 100% uptime.".into(),
                    description_id: "Cisco OpenDNS dengan proteksi phishing otomatis dan ketersediaan 100%.".into(),
                    tag_en: "ENTERPRISE".into(),
                    tag_id: "ENTERPRISE".into(),
                    ipv4_primary: "208.67.222.222".into(),
                    ipv4_secondary: "208.67.220.220".into(),
                    ipv6_primary: "2620:119:35::35".into(),
                    ipv6_secondary: "2620:119:53::53".into(),
                },
                DnsProfile {
                    id: "familyshield".into(),
                    name_en: "FamilyShield".into(),
                    name_id: "FamilyShield (Blokir Dewasa)".into(),
                    description_en: "Pre-configured to automatically block adult and illicit content out-of-the-box.".into(),
                    description_id: "Otomatis memblokir konten dewasa dan pornografi langsung tanpa konfigurasi tambahan.".into(),
                    tag_en: "FAMILY SAFE".into(),
                    tag_id: "RAMAH KELUARGA".into(),
                    ipv4_primary: "208.67.222.123".into(),
                    ipv4_secondary: "208.67.220.123".into(),
                    ipv6_primary: "2620:119:35::123".into(),
                    ipv6_secondary: "2620:119:53::123".into(),
                },
            ],
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
