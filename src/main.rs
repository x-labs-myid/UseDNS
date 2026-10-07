#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod booster;
#[cfg(any(windows, test))]
mod hover;
mod metrics;
mod models;
#[cfg(windows)]
mod native_windows;
mod startup;
mod storage;
mod system;
#[cfg(windows)]
mod tray;

use metrics::format_speed;
use models::DnsProvider;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

slint::include_modules!();

fn provider_symbol(id: &str, name: &str) -> String {
    match id {
        "cloudflare" => "CF".into(),
        "google" => "G".into(),
        "quad9" => "Q9".into(),
        "adguard" => "A".into(),
        "cleanbrowsing" => "CB".into(),
        "control-d" => "CD".into(),
        "nextdns" => "ND".into(),
        "opendns" => "OD".into(),
        _ => {
            let mut chars = name.split_whitespace().filter_map(|w| w.chars().next());
            match (chars.next(), chars.next()) {
                (Some(a), Some(b)) => format!("{}{}", a.to_uppercase(), b.to_uppercase()),
                (Some(a), None) => a.to_uppercase().to_string(),
                _ => "DNS".into(),
            }
        }
    }
}

fn provider_tag(provider: &DnsProvider, language: &str) -> String {
    if language == "id" {
        match provider.id.as_str() {
            "cloudflare" => "PRIVASI & CEPAT".into(),
            "google" => "KECEPATAN & ANDAL".into(),
            "quad9" => "KEAMANAN".into(),
            "adguard" => "BEBAS IKLAN".into(),
            "cleanbrowsing" => "KELUARGA & AMAN".into(),
            "control-d" => "BEBAS IKLAN & MALWARE".into(),
            "nextdns" => "PRIVASI CLOUD".into(),
            "opendns" => "KEAMANAN ENTERPRISE".into(),
            _ => match provider.purpose.as_str() {
                "privacy" => "PRIVASI".into(),
                "speed" => "KECEPATAN".into(),
                "security" => "KEAMANAN".into(),
                "ad-blocking" => "BEBAS IKLAN".into(),
                _ => "KUSTOM".into(),
            },
        }
    } else {
        match provider.id.as_str() {
            "cloudflare" => "PRIVACY & FAST".into(),
            "google" => "SPEED & RELIABLE".into(),
            "quad9" => "SECURITY".into(),
            "adguard" => "AD BLOCKING".into(),
            "cleanbrowsing" => "FAMILY SAFE".into(),
            "control-d" => "ADS & MALWARE".into(),
            "nextdns" => "CLOUD PRIVACY".into(),
            "opendns" => "ENTERPRISE SECURITY".into(),
            _ => match provider.purpose.as_str() {
                "privacy" => "PRIVACY".into(),
                "speed" => "SPEED".into(),
                "security" => "SECURITY".into(),
                "ad-blocking" => "AD BLOCKING".into(),
                _ => "CUSTOM".into(),
            },
        }
    }
}

fn dns_matches(current_dns: &str, address: &str) -> bool {
    let address = address.trim();
    if address.is_empty() {
        return false;
    }
    current_dns
        .split([',', ';', ' ', '\t'])
        .map(str::trim)
        .any(|part| part.eq_ignore_ascii_case(address))
}

fn resolve_active_provider(
    providers: &[DnsProvider],
    current_dns: &str,
    language: &str,
) -> (String, String) {
    for provider in providers {
        if !provider_is_active(provider, current_dns) {
            continue;
        }
        let profiles = provider.get_profiles();
        let title = if profiles.len() > 1 {
            profiles
                .iter()
                .find(|profile| {
                    [
                        profile.ipv4_primary.as_str(),
                        profile.ipv4_secondary.as_str(),
                        profile.ipv6_primary.as_str(),
                        profile.ipv6_secondary.as_str(),
                    ]
                    .into_iter()
                    .any(|address| dns_matches(current_dns, address))
                })
                .map(|profile| {
                    let profile_name = if language == "id" {
                        &profile.name_id
                    } else {
                        &profile.name_en
                    };
                    provider.name.clone() + " - " + profile_name
                })
                .unwrap_or_else(|| provider.name.clone())
        } else {
            provider.name.clone()
        };
        return (provider.id.clone(), title);
    }
    (
        "system".into(),
        if language == "id" {
            "Sistem / DHCP".into()
        } else {
            "System / DHCP".into()
        },
    )
}

fn apply_active_provider_state(
    window: &AppWindow,
    providers: &[DnsProvider],
    current_dns: &str,
    language: &str,
) {
    let (id, title) = resolve_active_provider(providers, current_dns, language);
    window.set_active_provider_id(id.into());
    window.set_active_provider(title.into());
}

fn provider_is_active(provider: &DnsProvider, current_dns: &str) -> bool {
    if current_dns.trim().is_empty() {
        return false;
    }
    provider.get_profiles().iter().any(|profile| {
        [
            profile.ipv4_primary.as_str(),
            profile.ipv4_secondary.as_str(),
            profile.ipv6_primary.as_str(),
            profile.ipv6_secondary.as_str(),
        ]
        .into_iter()
        .any(|address| dns_matches(current_dns, address))
    })
}

fn row(
    provider: &DnsProvider,
    language: &str,
    current_dns: &str,
    active_provider_id: &str,
) -> ProviderRow {
    let is_active = active_provider_id != "system"
        && provider.id == active_provider_id
        && provider_is_active(provider, current_dns);

    let profile_count = provider.get_profiles().len() as i32;

    ProviderRow {
        id: provider.id.as_str().into(),
        name: provider.name.as_str().into(),
        symbol: provider_symbol(&provider.id, &provider.name).into(),
        ipv4: format_pair(&provider.ipv4_primary, &provider.ipv4_secondary).into(),
        ipv6: format_pair(&provider.ipv6_primary, &provider.ipv6_secondary).into(),
        summary: if language == "id" {
            provider.summary_id.as_str()
        } else {
            provider.summary_en.as_str()
        }
        .into(),
        pros: if language == "id" {
            provider.pros_id.as_str()
        } else {
            provider.pros_en.as_str()
        }
        .into(),
        cons: if language == "id" {
            provider.cons_id.as_str()
        } else {
            provider.cons_en.as_str()
        }
        .into(),
        custom: provider.custom,
        active: is_active,
        tag: provider_tag(provider, language).into(),
        profile_count,
    }
}

fn doh_template(provider_id: &str, profile_id: &str) -> &'static str {
    match (provider_id, profile_id) {
        ("cloudflare", "standard") => "https://cloudflare-dns.com/dns-query",
        ("cloudflare", "security") => "https://security.cloudflare-dns.com/dns-query",
        ("cloudflare", "family") => "https://family.cloudflare-dns.com/dns-query",
        ("google", _) => "https://dns.google/dns-query",
        ("quad9", "standard") => "https://dns.quad9.net/dns-query",
        ("quad9", "ecs") => "https://dns11.quad9.net/dns-query",
        ("quad9", "unsecured") => "https://dns10.quad9.net/dns-query",
        ("adguard", "default") => "https://dns.adguard-dns.com/dns-query",
        ("adguard", "family") => "https://family.adguard-dns.com/dns-query",
        ("adguard", "non-filtering") => "https://unfiltered.adguard-dns.com/dns-query",
        ("cleanbrowsing", "family") => "https://doh.cleanbrowsing.org/doh/family-filter/",
        ("cleanbrowsing", "adult") => "https://doh.cleanbrowsing.org/doh/adult-filter/",
        ("cleanbrowsing", "security") => "https://doh.cleanbrowsing.org/doh/security-filter/",
        ("control-d", "unfiltered") => "https://freedns.controld.com/p0",
        ("control-d", "malware") => "https://freedns.controld.com/p1",
        ("control-d", "ads-malware") => "https://freedns.controld.com/p2",
        ("control-d", "family" | "social") => "https://freedns.controld.com/p3",
        // NextDNS DoH requires an account-specific configuration ID. The generic
        // IPv4 addresses cannot be registered as a Windows DoH server pair.
        ("nextdns", _) => "",
        ("opendns", "standard") => "https://doh.opendns.com/dns-query",
        ("opendns", "familyshield") => "https://doh.familyshield.opendns.com/dns-query",
        _ => "",
    }
}

fn format_pair(primary: &str, secondary: &str) -> String {
    match (primary.is_empty(), secondary.is_empty()) {
        (true, _) => "—".into(),
        (false, true) => primary.into(),
        (false, false) => format!("{primary}, {secondary}"),
    }
}

fn set_provider_model(
    window: &AppWindow,
    providers: &[DnsProvider],
    language: &str,
    current_dns: &str,
) {
    set_filtered_provider_model(window, providers, language, current_dns, "", "all");
}

fn set_filtered_provider_model(
    window: &AppWindow,
    providers: &[DnsProvider],
    language: &str,
    current_dns: &str,
    query: &str,
    category: &str,
) {
    let query_lower = query.trim().to_lowercase();
    let active_provider_id = window.get_active_provider_id().to_string();
    let rows = providers
        .iter()
        .filter(|provider| {
            let matches_query = query_lower.is_empty()
                || provider.name.to_lowercase().contains(&query_lower)
                || provider.summary_en.to_lowercase().contains(&query_lower)
                || provider.summary_id.to_lowercase().contains(&query_lower);
            let matches_category = match category {
                "all" => true,
                "privacy" => {
                    matches!(provider.id.as_str(), "cloudflare" | "quad9" | "nextdns")
                        || provider.purpose == "privacy"
                }
                "speed" => {
                    matches!(provider.id.as_str(), "cloudflare" | "google" | "opendns")
                        || provider.purpose == "speed"
                }
                "security" => {
                    matches!(
                        provider.id.as_str(),
                        "quad9" | "adguard" | "cleanbrowsing" | "opendns"
                    ) || provider.purpose == "security"
                }
                "ads" => {
                    matches!(provider.id.as_str(), "adguard" | "control-d" | "nextdns")
                        || provider.purpose == "ad-blocking"
                }
                _ => true,
            };
            matches_query && matches_category
        })
        .map(|provider| row(provider, language, current_dns, &active_provider_id))
        .collect::<Vec<_>>();
    window.set_providers(ModelRc::from(Rc::new(VecModel::from(rows))));
}

#[derive(Clone, Copy)]
struct TrayPreferences {
    start_with_windows: bool,
    network_interval_secs: u32,
    show_speed: bool,
    show_dns: bool,
    show_status: bool,
    show_latency: bool,
}

fn settings_snapshot(
    language: &str,
    theme: &str,
    providers: &[DnsProvider],
    tray: TrayPreferences,
) -> storage::Settings {
    storage::Settings {
        language: language.into(),
        theme: theme.into(),
        network_interval_secs: tray.network_interval_secs,
        tray_show_speed: tray.show_speed,
        tray_show_dns: tray.show_dns,
        tray_show_status: tray.show_status,
        tray_show_latency: tray.show_latency,
        start_with_windows: tray.start_with_windows,
        custom_providers: providers
            .iter()
            .filter(|provider| provider.custom)
            .cloned()
            .collect(),
    }
}

fn show_message(window: &AppWindow, message: impl Into<SharedString>, error: bool) {
    let message = message.into();
    let display_message = if error {
        friendly_error_message(window, message.as_str())
    } else {
        message.to_string()
    };
    window.set_toast_message(display_message.into());
    window.set_toast_error(error);
}

fn save_preferences(settings: &storage::Settings, weak: &slint::Weak<AppWindow>) {
    if let Err(error) = storage::save(settings)
        && let Some(window) = weak.upgrade()
    {
        show_message(&window, format!("Could not save settings: {error}"), true);
    }
}

fn friendly_error_message(window: &AppWindow, message: &str) -> String {
    let indonesian = window.get_language().as_str() == "id";
    let normalized = message.to_lowercase();

    if normalized.contains("cancelled")
        || normalized.contains("canceled")
        || normalized.contains("dibatalkan")
    {
        return if indonesian {
            "Izin Administrator dibatalkan. DNS tidak diubah."
        } else {
            "Administrator permission was cancelled. DNS was not changed."
        }
        .into();
    }

    if normalized.contains("administrator")
        || normalized.contains("access to a cim resource")
        || normalized.contains("access is denied")
        || normalized.contains("permission")
    {
        return if indonesian {
            "UseDNS memerlukan izin Administrator untuk mengubah DNS. Setujui dialog UAC lalu coba lagi."
        } else {
            "UseDNS needs Administrator permission to change DNS. Approve the UAC prompt and try again."
        }
        .into();
    }

    let first_line = message
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(if indonesian {
            "Operasi tidak dapat diselesaikan."
        } else {
            "The operation could not be completed."
        });
    let mut concise = first_line.chars().take(180).collect::<String>();
    if first_line.chars().count() > 180 {
        concise.push('…');
    }
    concise
}

#[cfg(windows)]
#[repr(C)]
struct Margins {
    cx_left_width: i32,
    cx_right_width: i32,
    cy_top_height: i32,
    cy_bottom_height: i32,
}

#[cfg(windows)]
#[link(name = "user32")]
#[link(name = "dwmapi")]
unsafe extern "system" {
    fn GetCursorPos(point: *mut windows::Win32::Foundation::POINT) -> i32;
    fn GetWindowRect(
        hwnd: *mut std::ffi::c_void,
        rect: *mut windows::Win32::Foundation::RECT,
    ) -> i32;
    fn ShowWindow(hwnd: *mut std::ffi::c_void, ncmdshow: i32) -> i32;
    fn DwmExtendFrameIntoClientArea(hwnd: *mut std::ffi::c_void, pmargins: *const Margins) -> i32;
    fn DwmSetWindowAttribute(
        hwnd: *mut std::ffi::c_void,
        dw_attribute: u32,
        pv_attribute: *const std::ffi::c_void,
        cb_attribute: u32,
    ) -> i32;
}

fn main() -> Result<(), slint::PlatformError> {
    if let Some(exit_code) = system::run_dns_helper_if_requested() {
        std::process::exit(exit_code);
    }

    #[cfg(windows)]
    let (window, tray_preview) = native_windows::create_windows()?;
    #[cfg(not(windows))]
    let (window, tray_preview) = (AppWindow::new()?, TrayPreview::new()?);

    #[cfg(windows)]
    let hwnd: Option<isize> = {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = window.window().window_handle().window_handle() {
            match handle.as_raw() {
                RawWindowHandle::Win32(win32) => Some(win32.hwnd.get()),
                _ => None,
            }
        } else {
            None
        }
    };
    #[cfg(not(windows))]
    let hwnd: Option<isize> = None;

    #[cfg(windows)]
    if let Some(h) = hwnd {
        let margins = Margins {
            cx_left_width: 1,
            cx_right_width: 1,
            cy_top_height: 1,
            cy_bottom_height: 1,
        };
        // SAFETY: h is a valid HWND pointer obtained from slint window handle,
        // margins is a valid stack-allocated Margins struct.
        unsafe {
            DwmExtendFrameIntoClientArea(h as _, &margins);

            // Set native Windows 11 rounded window corners (DWMWCP_ROUND = 2)
            let corner_preference: u32 = 2;
            DwmSetWindowAttribute(
                h as _,
                33, // DWMWA_WINDOW_CORNER_PREFERENCE
                &corner_preference as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }

    {
        use slint::winit_030::WinitWindowAccessor;

        let weak = window.as_weak();
        window.on_window_drag(move |kind| {
            if kind != 0 {
                return;
            }

            if let Some(window) = weak.upgrade() {
                window.window().with_winit_window(|winit_window| {
                    // Let winit hand the gesture to the platform. On Windows this
                    // preserves native moving, snapping, and maximized-window restore
                    // while guarding against duplicate drag requests.
                    let _ = winit_window.drag_window();
                });
            }
        });
    }

    {
        let weak = window.as_weak();
        window.on_window_minimize(move || {
            #[cfg(windows)]
            {
                let mut target_hwnd: *mut std::ffi::c_void = if let Some(h) = hwnd {
                    h as _
                } else {
                    std::ptr::null_mut()
                };

                if target_hwnd.is_null()
                    && let Some(w) = weak.upgrade()
                {
                    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                    if let Ok(handle) = w.window().window_handle().window_handle()
                        && let RawWindowHandle::Win32(win32) = handle.as_raw()
                    {
                        target_hwnd = win32.hwnd.get() as _;
                    }
                }

                if !target_hwnd.is_null() {
                    // SAFETY: target_hwnd is non-null and SW_MINIMIZE is a valid command for ShowWindow.
                    unsafe {
                        ShowWindow(target_hwnd, 6 /* SW_MINIMIZE */);
                    }
                } else if let Some(window) = weak.upgrade() {
                    window.window().set_minimized(true);
                }
            }
            #[cfg(not(windows))]
            if let Some(w) = weak.upgrade() {
                w.window().set_minimized(true);
            }
        });
    }

    {
        let weak = window.as_weak();
        window.window().on_close_requested(move || {
            if let Some(window) = weak.upgrade() {
                window.invoke_window_close();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    }

    {
        let weak = window.as_weak();
        window.on_window_close(move || {
            if let Some(window) = weak.upgrade() {
                window.set_nav_menu_open(false);
                window.set_close_dialog_open(true);
            }
        });
    }
    {
        let weak = window.as_weak();
        let preview = tray_preview.as_weak();
        window.on_close_choice(move |background| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if window.get_busy() {
                return;
            }
            window.set_close_dialog_open(false);
            if let Some(preview) = preview.upgrade() {
                let _ = preview.hide();
            }
            if background && window.get_tray_available() {
                let _ = window.hide();
            } else if !background {
                let _ = slint::quit_event_loop();
            }
        });
    }

    let mut settings = storage::load();
    #[cfg(windows)]
    if let Ok(enabled) = startup::is_enabled() {
        settings.start_with_windows = enabled;
    }
    if settings.language != "id" {
        settings.language = "en".into();
    }
    if !matches!(settings.theme.as_str(), "system" | "light" | "dark") {
        settings.theme = "system".into();
    }
    if !matches!(settings.network_interval_secs, 1 | 2 | 5) {
        settings.network_interval_secs = 1;
    }
    let tray_preferences = Rc::new(RefCell::new(TrayPreferences {
        start_with_windows: settings.start_with_windows,
        network_interval_secs: settings.network_interval_secs,
        show_speed: settings.tray_show_speed,
        show_dns: settings.tray_show_dns,
        show_status: settings.tray_show_status,
        show_latency: settings.tray_show_latency,
    }));

    let mut initial = models::default_providers();
    for mut provider in settings.custom_providers {
        if provider.validate().is_ok()
            && !provider.id.is_empty()
            && !initial.iter().any(|existing| existing.id == provider.id)
        {
            provider.custom = true;
            initial.push(provider);
        }
    }
    let providers = Rc::new(RefCell::new(initial));
    let language = Rc::new(RefCell::new(settings.language));
    let theme = Rc::new(RefCell::new(settings.theme));
    let adapters = Arc::new(Mutex::new(Vec::<system::AdapterInfo>::new()));
    let provider_filter = Rc::new(RefCell::new((String::new(), String::from("all"))));

    window.set_system_dark(system::prefers_dark_mode());
    window.set_language(language.borrow().as_str().into());
    window.set_theme_mode(theme.borrow().as_str().into());
    window.set_network_interval_secs(settings.network_interval_secs as i32);
    window.set_tray_show_speed(settings.tray_show_speed);
    window.set_tray_show_dns(settings.tray_show_dns);
    window.set_tray_show_status(settings.tray_show_status);
    window.set_tray_show_latency(settings.tray_show_latency);
    window.set_start_with_windows(settings.start_with_windows);
    window.set_startup_supported(cfg!(windows));
    tray_preview.set_theme_mode(theme.borrow().as_str().into());
    tray_preview.set_system_dark(window.get_system_dark());
    tray_preview.set_show_speed(settings.tray_show_speed);
    tray_preview.set_show_dns(settings.tray_show_dns);
    tray_preview.set_show_status(settings.tray_show_status);
    tray_preview.set_show_latency(settings.tray_show_latency);
    set_provider_model(&window, &providers.borrow(), &language.borrow(), "");

    {
        let weak = window.as_weak();
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let preferences = tray_preferences.clone();
        window.on_startup_changed(move |enabled| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if window.get_busy() {
                return;
            }
            let previous = preferences.borrow().start_with_windows;
            if let Err(error) = startup::set_enabled(enabled) {
                show_message(&window, error, true);
                return;
            }
            let mut updated = *preferences.borrow();
            updated.start_with_windows = enabled;
            let settings = settings_snapshot(
                &language.borrow(),
                &theme.borrow(),
                &providers.borrow(),
                updated,
            );
            if let Err(error) = storage::save(&settings) {
                let rollback = startup::set_enabled(previous);
                let message = match rollback {
                    Ok(()) => format!("Could not save settings: {error}"),
                    Err(rollback) => format!(
                        "Could not save settings: {error}. Startup rollback failed: {rollback}"
                    ),
                };
                show_message(&window, message, true);
                return;
            }
            *preferences.borrow_mut() = updated;
            window.set_start_with_windows(enabled);
        });
    }
    {
        let weak = window.as_weak();
        let providers = providers.clone();
        window.on_run_booster(move || {
            let Some(window) = weak.upgrade() else { return; };
            if window.get_booster_running() || window.get_busy() || window.get_page() != 1 { return; }
            let generation = window.get_booster_generation().wrapping_add(1);
            window.set_booster_generation(generation);
            let indonesian = window.get_language() == "id";
            let mut candidates = Vec::new();
            if let Some(address) = window.get_current_dns().split([',', ';', ' ', '\t'])
                .find_map(|part| part.trim().parse::<std::net::Ipv4Addr>().ok()) {
                candidates.push(booster::Candidate {
                    provider_id: String::new(),
                    name: if indonesian { "DNS saat ini" } else { "Current DNS" }.into(),
                    address: address.into(),
                });
            }
            for provider in providers.borrow().iter().filter(|p| !p.custom) {
                if let Some(profile) = provider.get_profiles().first()
                    && let Ok(address) = profile.ipv4_primary.parse::<std::net::Ipv4Addr>() {
                    if candidates.iter().any(|candidate| candidate.address == std::net::IpAddr::V4(address)) {
                        continue;
                    }
                    candidates.push(booster::Candidate { provider_id: provider.id.clone(),
                        name: provider.name.clone(), address: address.into() });
                }
            }
            window.set_booster_running(true);
            window.set_booster_provider_id("".into());
            window.set_booster_results("".into());
            window.set_booster_rows(ModelRc::new(VecModel::<DnsScanRow>::default()));
            window.set_booster_current_best(false);
            window.set_booster_completed(0);
            window.set_booster_total(candidates.len() as i32);
            let weak = weak.clone();
            std::thread::spawn(move || {
                let progress_weak = weak.clone();
                let results = booster::benchmark(candidates, move |completed| {
                    let weak = progress_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = weak.upgrade()
                            && window.get_booster_generation() == generation
                            && window.get_page() == 1 {
                            window.set_booster_completed(completed as i32);
                        }
                    });
                });
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(window) = weak.upgrade() else { return; };
                    if window.get_booster_generation() != generation || window.get_page() != 1 {
                        return;
                    }
                    window.set_booster_running(false);
                    if let Some(candidate) = booster::review_candidate(&results) {
                        window.set_booster_provider_id(candidate.provider_id.as_str().into());
                    }
                    let summary;
                    if let Some(best) = results.first().filter(|r| r.median_ms.is_some()) {
                        if best.candidate.provider_id.is_empty() {
                            window.set_booster_current_best(true);
                            summary = if indonesian { "DNS saat ini unggul pada pengujian ini" }
                                else { "Current DNS leads in this test" }.to_string();
                        } else {
                            summary = format!("{}: {}", if indonesian { "Rekomendasi" } else { "Recommendation" }, best.candidate.name);
                        }
                    } else {
                        summary = if indonesian { "Belum ada DNS yang lolos pengujian. Coba pindai ulang." }
                            else { "No DNS qualified in this test. Try scanning again." }.to_string();
                    }
                    let rows = results.iter().enumerate().map(|(index, result)| DnsScanRow {
                        name: result.candidate.name.as_str().into(),
                        address: result.candidate.address.to_string().into(),
                        latency: result.median_ms.map(|ms| format!("{ms:.1} ms"))
                            .unwrap_or_else(|| "—".into()).into(),
                        successful: result.successful as i32,
                        qualified: result.median_ms.is_some(),
                        best: index == 0 && result.median_ms.is_some(),
                        current: result.candidate.provider_id.is_empty(),
                    }).collect::<Vec<_>>();
                    window.set_booster_rows(ModelRc::new(VecModel::from(rows)));
                    window.set_booster_results(summary.into());
                });
            });
        });
    }
    {
        let weak = window.as_weak();
        window.on_review_booster(move || {
            if let Some(window) = weak.upgrade()
                && !window.get_busy()
            {
                let provider = window.get_booster_provider_id();
                if !provider.is_empty() {
                    window.invoke_navigate(1);
                    window.invoke_request_use_provider(provider);
                }
            }
        });
    }

    {
        let weak = window.as_weak();
        let adapters = adapters.clone();
        let providers = providers.clone();
        let language = language.clone();
        let provider_filter = provider_filter.clone();
        let refreshing = Arc::new(AtomicBool::new(false));
        window.on_refresh(move || {
            if weak.upgrade().is_none_or(|window| window.get_busy()) {
                return;
            }
            if refreshing.swap(true, Ordering::AcqRel) {
                return;
            }
            let weak = weak.clone();
            let refreshing = refreshing.clone();
            let adapters = adapters.clone();
            let providers_snapshot = providers.borrow().clone();
            let language_value = language.borrow().clone();
            let (query, category) = provider_filter.borrow().clone();
            if let Some(window) = weak.upgrade() {
                window.set_refreshing(true);
                window.set_toast_message("".into());
            }
            std::thread::spawn(move || {
                let result = system::active_adapters();
                let _ = slint::invoke_from_event_loop(move || {
                    refreshing.store(false, Ordering::Release);
                    let Some(window) = weak.upgrade() else {
                        return;
                    };
                    match result {
                        Ok(found) => {
                            let labels = found
                                .iter()
                                .map(|item| SharedString::from(&item.label))
                                .collect::<Vec<_>>();
                            window.set_adapters(ModelRc::from(Rc::new(VecModel::from(labels))));
                            let index = (window.get_adapter_index().max(0) as usize)
                                .min(found.len().saturating_sub(1));
                            window.set_adapter_index(index as i32);
                            let selected = found.get(index).or_else(|| found.first());
                            if let Some(selected) = selected {
                                window.set_current_dns(selected.dns.clone().into());
                                let configured_dns = if selected.dns_automatic {
                                    ""
                                } else {
                                    selected.dns.as_str()
                                };
                                apply_active_provider_state(
                                    &window,
                                    &providers_snapshot,
                                    configured_dns,
                                    &language_value,
                                );
                                set_filtered_provider_model(
                                    &window,
                                    &providers_snapshot,
                                    &language_value,
                                    configured_dns,
                                    &query,
                                    &category,
                                );
                                window.set_refreshing(false);
                                window.invoke_check_connection();
                            } else {
                                window.set_refreshing(false);
                                window.set_connection_state("Unavailable".into());
                                window.set_latency("—".into());
                            }
                            if let Ok(mut cached) = adapters.lock() {
                                *cached = found;
                            }
                        }
                        Err(message) => {
                            window.set_refreshing(false);
                            window.set_connection_state("Unavailable".into());
                            window.set_latency("—".into());
                            show_message(&window, message, true);
                        }
                    }
                });
            });
        });
    }

    {
        let weak = window.as_weak();
        let checking = Arc::new(AtomicBool::new(false));
        window.on_check_connection(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if window.get_busy() || window.get_refreshing() || checking.swap(true, Ordering::AcqRel)
            {
                return;
            }
            let dns = window.get_current_dns().to_string();
            if window.get_adapters().row_count() == 0 {
                checking.store(false, Ordering::Release);
                window.set_connection_state("Unavailable".into());
                window.set_latency("—".into());
                return;
            }
            if window.get_latency() == "—" {
                window.set_connection_state("Checking...".into());
            }
            let weak = weak.clone();
            let checking = checking.clone();
            std::thread::spawn(move || {
                let result = system::check_connection(&dns);
                let _ = slint::invoke_from_event_loop(move || {
                    checking.store(false, Ordering::Release);
                    let Some(window) = weak.upgrade() else {
                        return;
                    };
                    // Discard results for an adapter/DNS configuration changed during the probe.
                    if window.get_current_dns().as_str() != dns
                        || window.get_busy()
                        || window.get_refreshing()
                    {
                        return;
                    }
                    match result {
                        Ok(ms) => {
                            window.set_latency(format!("{ms} ms").into());
                            window.set_connection_state(metrics::response_state(ms).into());
                        }
                        Err(_) => {
                            window.set_latency("—".into());
                            window.set_connection_state("Unavailable".into());
                            // A background diagnostic is not a failed user operation.
                        }
                    }
                });
            });
        });
    }
    let connection_timer = slint::Timer::default();
    {
        let weak = window.as_weak();
        connection_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_secs(15),
            move || {
                if let Some(window) = weak.upgrade() {
                    window.invoke_check_connection();
                }
            },
        );
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let tray_preferences = tray_preferences.clone();
        let weak = window.as_weak();
        let provider_filter = provider_filter.clone();
        window.on_language_changed(move |new_language| {
            let value = if new_language.as_str() == "id" {
                "id"
            } else {
                "en"
            };
            *language.borrow_mut() = value.into();
            if let Some(window) = weak.upgrade() {
                let (query, category) = provider_filter.borrow().clone();
                set_filtered_provider_model(
                    &window,
                    &providers.borrow(),
                    value,
                    &window.get_current_dns(),
                    &query,
                    &category,
                );
            }
            let settings = settings_snapshot(
                value,
                &theme.borrow(),
                &providers.borrow(),
                *tray_preferences.borrow(),
            );
            save_preferences(&settings, &weak);
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let tray_preferences = tray_preferences.clone();
        let weak = window.as_weak();
        window.on_theme_changed(move |new_theme| {
            let value = match new_theme.as_str() {
                "light" => "light",
                "dark" => "dark",
                _ => "system",
            };
            *theme.borrow_mut() = value.into();
            if value == "system"
                && let Some(window) = weak.upgrade()
            {
                window.set_system_dark(system::prefers_dark_mode());
            }
            let settings = settings_snapshot(
                &language.borrow(),
                value,
                &providers.borrow(),
                *tray_preferences.borrow(),
            );
            save_preferences(&settings, &weak);
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let tray_preferences = tray_preferences.clone();
        let weak = window.as_weak();
        window.on_network_interval_changed(move |seconds| {
            let seconds = match seconds {
                2 => 2,
                5 => 5,
                _ => 1,
            };
            tray_preferences.borrow_mut().network_interval_secs = seconds;
            let settings = settings_snapshot(
                &language.borrow(),
                &theme.borrow(),
                &providers.borrow(),
                *tray_preferences.borrow(),
            );
            save_preferences(&settings, &weak);
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let tray_preferences = tray_preferences.clone();
        let preview_weak = tray_preview.as_weak();
        let weak = window.as_weak();
        window.on_tray_preview_settings_changed(move |speed, dns, status, latency| {
            let preferences = TrayPreferences {
                start_with_windows: tray_preferences.borrow().start_with_windows,
                network_interval_secs: tray_preferences.borrow().network_interval_secs,
                show_speed: speed,
                show_dns: dns,
                show_status: status,
                show_latency: latency,
            };
            *tray_preferences.borrow_mut() = preferences;
            if let Some(preview) = preview_weak.upgrade() {
                preview.set_show_speed(speed);
                preview.set_show_dns(dns);
                preview.set_show_status(status);
                preview.set_show_latency(latency);
            }
            let settings = settings_snapshot(
                &language.borrow(),
                &theme.borrow(),
                &providers.borrow(),
                preferences,
            );
            save_preferences(&settings, &weak);
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let weak = window.as_weak();
        let provider_filter = provider_filter.clone();
        window.on_filter_providers(move |query, category| {
            *provider_filter.borrow_mut() = (query.to_string(), category.to_string());
            if let Some(window) = weak.upgrade() {
                set_filtered_provider_model(
                    &window,
                    &providers.borrow(),
                    &language.borrow(),
                    &window.get_current_dns(),
                    &query,
                    &category,
                );
            }
        });
    }

    {
        let providers = providers.clone();
        let adapters = adapters.clone();
        let language = language.clone();
        let weak = window.as_weak();
        window.on_request_use_provider(move |id| {
            let Some(provider) = providers
                .borrow()
                .iter()
                .find(|p| p.id == id.as_str())
                .cloned()
            else {
                return;
            };
            let lang = language.borrow().clone();
            let profiles = provider.get_profiles();
            let profile_rows = profiles
                .iter()
                .map(|p| {
                    let name = if lang == "id" { &p.name_id } else { &p.name_en };
                    let description = if lang == "id" {
                        &p.description_id
                    } else {
                        &p.description_en
                    };
                    let tag = if lang == "id" { &p.tag_id } else { &p.tag_en };
                    DnsProfileRow {
                        id: p.id.as_str().into(),
                        name: name.as_str().into(),
                        description: description.as_str().into(),
                        tag: tag.as_str().into(),
                        ipv4: format_pair(&p.ipv4_primary, &p.ipv4_secondary).into(),
                        ipv6: format_pair(&p.ipv6_primary, &p.ipv6_secondary).into(),
                        doh_template: doh_template(&provider.id, &p.id).into(),
                    }
                })
                .collect::<Vec<_>>();
            let encrypted_available = profile_rows
                .first()
                .is_some_and(|profile| !profile.doh_template.is_empty());

            if let Some(window) = weak.upgrade() {
                let is_active_provider = window.get_active_provider_id().as_str() == provider.id;
                let adapter_encrypted = adapters
                    .lock()
                    .ok()
                    .and_then(|items| {
                        items
                            .get(window.get_adapter_index().max(0) as usize)
                            .map(|adapter| adapter.dns_encrypted)
                    })
                    .unwrap_or(false);
                window.set_popup_provider_id(provider.id.into());
                window.set_popup_provider_name(provider.name.into());
                window.set_popup_profiles(ModelRc::from(Rc::new(VecModel::from(profile_rows))));
                window.set_selected_profile_index(0);
                window.set_encrypted_dns(if is_active_provider {
                    encrypted_available && adapter_encrypted
                } else {
                    encrypted_available
                });
                window.set_ip_mode(0);
                window.set_profile_popup_open(true);
            }
        });
    }

    {
        let providers = providers.clone();
        let adapters = adapters.clone();
        let language = language.clone();
        let provider_filter = provider_filter.clone();
        let weak = window.as_weak();
        window.on_confirm_apply_profile(
            move |id, profile_index, adapter_index, mode, encrypted, doh_template| {
                if weak
                    .upgrade()
                    .is_none_or(|window| window.get_busy() || window.get_refreshing())
                {
                    return;
                }
                let provider = providers
                    .borrow()
                    .iter()
                    .find(|p| p.id == id.as_str())
                    .cloned();
                let adapter = adapters
                    .lock()
                    .ok()
                    .and_then(|items| items.get(adapter_index.max(0) as usize).cloned());
                let (Some(provider), Some(adapter)) = (provider, adapter) else {
                    if let Some(window) = weak.upgrade() {
                        show_message(&window, "Select an active network adapter first.", true);
                    }
                    return;
                };

                let profiles = provider.get_profiles();
                let selected_profile = profiles
                    .get(profile_index.max(0) as usize)
                    .or_else(|| profiles.first());
                let Some(profile) = selected_profile else {
                    return;
                };

                let mut addresses = Vec::new();
                if mode == 0 || mode == 2 {
                    addresses.extend(
                        [profile.ipv4_primary.clone(), profile.ipv4_secondary.clone()]
                            .into_iter()
                            .filter(|v| !v.is_empty()),
                    );
                }
                if mode == 1 || mode == 2 {
                    addresses.extend(
                        [profile.ipv6_primary.clone(), profile.ipv6_secondary.clone()]
                            .into_iter()
                            .filter(|v| !v.is_empty()),
                    );
                }
                if addresses.is_empty() {
                    if let Some(window) = weak.upgrade() {
                        show_message(
                            &window,
                            "This profile has no address for the selected IP mode.",
                            true,
                        );
                    }
                    return;
                }
                if encrypted && doh_template.trim().is_empty() {
                    if let Some(window) = weak.upgrade() {
                        show_message(
                            &window,
                            if window.get_language().as_str() == "id" {
                                "Profil ini tidak memiliki template DNS terenkripsi."
                            } else {
                                "This profile does not provide an encrypted DNS template."
                            },
                            true,
                        );
                    }
                    return;
                }

                if let Some(window) = weak.upgrade() {
                    window.set_busy(true);
                    window.set_toast_message("".into());
                }
                let weak = weak.clone();
                let lang = language.borrow().clone();
                let profile_display_name = if lang == "id" {
                    profile.name_id.clone()
                } else {
                    profile.name_en.clone()
                };
                let active_title = if profiles.len() > 1 {
                    provider.name.clone() + " - " + &profile_display_name
                } else {
                    provider.name.clone()
                };
                let active_provider_id = provider.id.clone();

                let providers_snapshot = providers.borrow().clone();
                let (query, category) = provider_filter.borrow().clone();
                let adapter_state = adapters.clone();
                std::thread::spawn(move || {
                    let result = system::apply_dns(
                        &adapter.name,
                        &addresses,
                        encrypted.then_some(doh_template.as_str()),
                    );
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = weak.upgrade() {
                            window.set_busy(false);
                            match result {
                                Ok(()) => {
                                    if let Ok(mut items) = adapter_state.lock()
                                        && let Some(item) =
                                            items.iter_mut().find(|item| item.name == adapter.name)
                                    {
                                        item.dns = addresses.join(", ");
                                        item.dns_automatic = false;
                                        item.dns_encrypted = encrypted;
                                    }
                                    window.set_profile_popup_open(false);
                                    window.set_active_provider_id(active_provider_id.into());
                                    window.set_active_provider(active_title.into());
                                    window.set_current_dns(addresses.join(", ").into());
                                    window.invoke_check_connection();
                                    set_filtered_provider_model(
                                        &window,
                                        &providers_snapshot,
                                        &lang,
                                        &addresses.join(", "),
                                        &query,
                                        &category,
                                    );
                                    show_message(
                                        &window,
                                        if lang == "id" {
                                            "DNS berhasil diterapkan."
                                        } else {
                                            "DNS applied successfully."
                                        },
                                        false,
                                    );
                                }
                                Err(message) => show_message(&window, message, true),
                            }
                        }
                    });
                });
            },
        );
    }

    {
        let providers = providers.clone();
        let adapters = adapters.clone();
        let language = language.clone();
        let weak = window.as_weak();
        window.on_apply_selected(move |id, adapter_index, mode| {
            if weak
                .upgrade()
                .is_none_or(|window| window.get_busy() || window.get_refreshing())
            {
                return;
            }
            let provider = providers
                .borrow()
                .iter()
                .find(|provider| provider.id == id.as_str())
                .cloned();
            let adapter = adapters
                .lock()
                .ok()
                .and_then(|items| items.get(adapter_index.max(0) as usize).cloned());
            let (Some(provider), Some(adapter)) = (provider, adapter) else {
                if let Some(window) = weak.upgrade() {
                    show_message(&window, "Select an active network adapter first.", true);
                }
                return;
            };
            let mut addresses = Vec::new();
            if mode == 0 || mode == 2 {
                addresses.extend(
                    [
                        provider.ipv4_primary.clone(),
                        provider.ipv4_secondary.clone(),
                    ]
                    .into_iter()
                    .filter(|value| !value.is_empty()),
                );
            }
            if mode == 1 || mode == 2 {
                addresses.extend(
                    [
                        provider.ipv6_primary.clone(),
                        provider.ipv6_secondary.clone(),
                    ]
                    .into_iter()
                    .filter(|value| !value.is_empty()),
                );
            }
            if addresses.is_empty() {
                if let Some(window) = weak.upgrade() {
                    show_message(
                        &window,
                        "This provider has no address for the selected IP mode.",
                        true,
                    );
                }
                return;
            }
            if let Some(window) = weak.upgrade() {
                window.set_busy(true);
                window.set_toast_message("".into());
            }
            let weak = weak.clone();
            let provider_name = provider.name.clone();
            let provider_id = provider.id.clone();
            let language_value = language.borrow().clone();
            let adapter_state = adapters.clone();
            std::thread::spawn(move || {
                let result = system::apply_dns(&adapter.name, &addresses, None);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
                        match result {
                            Ok(()) => {
                                if let Ok(mut items) = adapter_state.lock()
                                    && let Some(item) =
                                        items.iter_mut().find(|item| item.name == adapter.name)
                                {
                                    item.dns = addresses.join(", ");
                                    item.dns_automatic = false;
                                    item.dns_encrypted = false;
                                }
                                window.set_active_provider_id(provider_id.into());
                                window.set_active_provider(provider_name.clone().into());
                                window.set_current_dns(addresses.join(", ").into());
                                window.invoke_check_connection();
                                show_message(
                                    &window,
                                    if language_value == "id" {
                                        "DNS berhasil diterapkan."
                                    } else {
                                        "DNS applied successfully."
                                    },
                                    false,
                                );
                            }
                            Err(message) => show_message(&window, message, true),
                        }
                    }
                });
            });
        });
    }

    {
        let adapters = adapters.clone();
        let language = language.clone();
        let weak = window.as_weak();
        window.on_reset_dns(move |adapter_index| {
            if weak
                .upgrade()
                .is_none_or(|window| window.get_busy() || window.get_refreshing())
            {
                return;
            }
            let adapter = adapters
                .lock()
                .ok()
                .and_then(|items| items.get(adapter_index.max(0) as usize).cloned());
            let Some(adapter) = adapter else {
                if let Some(window) = weak.upgrade() {
                    show_message(&window, "Select an active network adapter first.", true);
                }
                return;
            };
            if let Some(window) = weak.upgrade() {
                window.set_busy(true);
            }
            let weak = weak.clone();
            let language = language.borrow().clone();
            std::thread::spawn(move || {
                let result = system::reset_dns(&adapter.name);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
                        match result {
                            Ok(()) => {
                                window.set_active_provider_id("system".into());
                                window.set_active_provider(
                                    if language == "id" {
                                        "Sistem / DHCP"
                                    } else {
                                        "System / DHCP"
                                    }
                                    .into(),
                                );
                                window.invoke_refresh();
                                show_message(
                                    &window,
                                    if language == "id" {
                                        "DNS otomatis berhasil dipulihkan."
                                    } else {
                                        "Automatic DNS restored."
                                    },
                                    false,
                                );
                            }
                            Err(message) => show_message(&window, message, true),
                        }
                    }
                });
            });
        });
    }

    {
        let weak = window.as_weak();
        let language = language.clone();
        window.on_test_resolver(move |ipv4, ipv6| {
            if weak
                .upgrade()
                .is_none_or(|window| window.get_testing_resolver())
            {
                return;
            }
            let address = if !ipv4.trim().is_empty() {
                ipv4.trim().to_string()
            } else {
                ipv6.trim().to_string()
            };
            if address.is_empty() {
                if let Some(window) = weak.upgrade() {
                    show_message(
                        &window,
                        if *language.borrow() == "id" {
                            "Masukkan alamat DNS utama sebelum melakukan pengujian."
                        } else {
                            "Enter a primary DNS address before testing."
                        },
                        true,
                    );
                }
                return;
            }
            if address.parse::<std::net::IpAddr>().is_err() {
                if let Some(window) = weak.upgrade() {
                    show_message(
                        &window,
                        if *language.borrow() == "id" {
                            "Alamat DNS tidak valid."
                        } else {
                            "The DNS address is invalid."
                        },
                        true,
                    );
                }
                return;
            }
            if let Some(window) = weak.upgrade() {
                window.set_testing_resolver(true);
                window.set_toast_message("".into());
            }
            let weak = weak.clone();
            let language = language.borrow().clone();
            std::thread::spawn(move || {
                let result = system::check_connection(&address);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_testing_resolver(false);
                        match result {
                            Ok(ms) => show_message(
                                &window,
                                if language == "id" {
                                    format!("Resolver dapat dijangkau dalam sekitar {ms} ms.")
                                } else {
                                    format!("Resolver is reachable in approximately {ms} ms.")
                                },
                                false,
                            ),
                            Err(message) => show_message(&window, message, true),
                        }
                    }
                });
            });
        });
    }

    {
        let weak = window.as_weak();
        window.on_clear_form(move || {
            if let Some(window) = weak.upgrade() {
                window.set_form_id("".into());
                window.set_form_name("".into());
                window.set_form_ipv4_primary("".into());
                window.set_form_ipv4_secondary("".into());
                window.set_form_ipv6_primary("".into());
                window.set_form_ipv6_secondary("".into());
                window.set_form_summary("".into());
                window.set_form_purpose("general".into());
            }
        });
    }

    {
        let providers = providers.clone();
        let weak = window.as_weak();
        window.on_edit_provider(move |id| {
            let Some(provider) = providers
                .borrow()
                .iter()
                .find(|provider| provider.id == id.as_str() && provider.custom)
                .cloned()
            else {
                return;
            };
            if let Some(window) = weak.upgrade() {
                window.set_form_id(provider.id.into());
                window.set_form_name(provider.name.into());
                window.set_form_ipv4_primary(provider.ipv4_primary.into());
                window.set_form_ipv4_secondary(provider.ipv4_secondary.into());
                window.set_form_ipv6_primary(provider.ipv6_primary.into());
                window.set_form_ipv6_secondary(provider.ipv6_secondary.into());
                window.set_form_summary(provider.summary_en.into());
                window.set_form_purpose(if provider.purpose.is_empty() {
                    "general".into()
                } else {
                    provider.purpose.into()
                });
            }
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let tray_preferences = tray_preferences.clone();
        let weak = window.as_weak();
        window.on_save_provider(move |id, name, v4a, v4b, v6a, v6b, summary, purpose| {
            let generated_id = if id.is_empty() {
                "custom-".to_owned()
                    + &std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                        .to_string()
            } else {
                id.to_string()
            };
            let provider = DnsProvider {
                id: generated_id.clone(),
                name: name.trim().into(),
                ipv4_primary: v4a.trim().into(),
                ipv4_secondary: v4b.trim().into(),
                ipv6_primary: v6a.trim().into(),
                ipv6_secondary: v6b.trim().into(),
                summary_en: summary.trim().into(),
                summary_id: summary.trim().into(),
                purpose: purpose.as_str().into(),
                pros_en: "User-defined resolver".into(),
                pros_id: "Resolver ditentukan pengguna".into(),
                cons_en: "Trust and availability depend on its operator".into(),
                cons_id: "Kepercayaan dan ketersediaan bergantung pada pengelolanya".into(),
                custom: true,
                profiles: Vec::new(),
            };
            if let Err(message) = provider.validate() {
                if let Some(window) = weak.upgrade() {
                    show_message(&window, message, true);
                }
                return;
            }
            let mut list = providers.borrow().clone();
            if let Some(existing) = list
                .iter_mut()
                .find(|item| item.id == generated_id && item.custom)
            {
                *existing = provider;
            } else {
                list.push(provider);
            }
            let settings = settings_snapshot(
                &language.borrow(),
                &theme.borrow(),
                &list,
                *tray_preferences.borrow(),
            );
            if let Err(error) = storage::save(&settings) {
                if let Some(window) = weak.upgrade() {
                    show_message(&window, format!("Could not save settings: {error}"), true);
                }
                return;
            }
            if let Some(window) = weak.upgrade() {
                set_provider_model(
                    &window,
                    &list,
                    &language.borrow(),
                    &window.get_current_dns(),
                );
                window.invoke_navigate(1);
                show_message(
                    &window,
                    if *language.borrow() == "id" {
                        "DNS kustom berhasil disimpan."
                    } else {
                        "Custom DNS saved."
                    },
                    false,
                );
            }
            *providers.borrow_mut() = list;
            if let Some(window) = weak.upgrade() {
                window.set_tray_menu_revision(window.get_tray_menu_revision().wrapping_add(1));
            }
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let tray_preferences = tray_preferences.clone();
        let weak = window.as_weak();
        window.on_delete_provider(move |id| {
            let mut list = providers.borrow().clone();
            list.retain(|provider| !provider.custom || provider.id != id.as_str());
            let settings = settings_snapshot(
                &language.borrow(),
                &theme.borrow(),
                &list,
                *tray_preferences.borrow(),
            );
            let result = storage::save(&settings);
            if let Err(error) = result {
                if let Some(window) = weak.upgrade() {
                    show_message(&window, format!("Could not save settings: {error}"), true);
                }
                return;
            }
            *providers.borrow_mut() = list.clone();
            if let Some(window) = weak.upgrade() {
                window.set_tray_menu_revision(window.get_tray_menu_revision().wrapping_add(1));
                set_provider_model(
                    &window,
                    &list,
                    &language.borrow(),
                    &window.get_current_dns(),
                );
                show_message(
                    &window,
                    if *language.borrow() == "id" {
                        "DNS kustom dihapus."
                    } else {
                        "Custom DNS deleted."
                    },
                    false,
                );
            }
        });
    }

    let network_timer = slint::Timer::default();
    {
        let weak = window.as_weak();
        let preview_weak = tray_preview.as_weak();
        let adapters = adapters.clone();
        let sampling_preferences = tray_preferences.clone();
        let sampling_ticks = Rc::new(std::cell::Cell::new(0_u32));
        let sampling_ticks_for_timer = sampling_ticks.clone();
        // Initialize speed-history with 24 blank samples
        let initial_samples: Vec<SpeedSample> = (0..24)
            .map(|_| SpeedSample {
                download: 0.0,
                upload: 0.0,
            })
            .collect();
        window.set_speed_history(ModelRc::from(Rc::new(VecModel::from(initial_samples))));
        window.set_graph_max_speed(format_speed(1.0).into());
        window.set_graph_mid_speed(format_speed(0.5).into());

        let (sample_sender, sample_receiver) = std::sync::mpsc::sync_channel::<Option<u32>>(1);
        let worker_weak = weak.clone();
        let worker_preview_weak = preview_weak.clone();
        std::thread::spawn(move || {
            let mut previous = None::<(u32, u64, u64, Instant)>;
            let mut samples = vec![(0.0f64, 0.0f64); 24];

            while let Ok(request) = sample_receiver.recv() {
                let Some(interface_index) = request else {
                    previous = None;
                    continue;
                };
                let Ok((received, sent)) = system::network_counters(interface_index) else {
                    continue;
                };

                let now = Instant::now();
                let (download, upload) = previous
                    .as_ref()
                    .filter(|(old_index, old_received, old_sent, _)| {
                        *old_index == interface_index
                            && received >= *old_received
                            && sent >= *old_sent
                    })
                    .map(|(_, old_received, old_sent, sampled_at)| {
                        let seconds = now.duration_since(*sampled_at).as_secs_f64().max(0.001);
                        (
                            (received - old_received) as f64 * 8.0 / seconds / 1_000_000.0,
                            (sent - old_sent) as f64 * 8.0 / seconds / 1_000_000.0,
                        )
                    })
                    .unwrap_or((0.0, 0.0));
                previous = Some((interface_index, received, sent, now));

                samples.rotate_left(1);
                if let Some(last) = samples.last_mut() {
                    *last = (download, upload);
                }
                let peak_mbps = samples
                    .iter()
                    .map(|(down, up)| down.max(*up))
                    .fold(0.5f64, f64::max);
                let max_scale = peak_mbps * 1.15;
                let speed_models = samples
                    .iter()
                    .map(|(down, up)| SpeedSample {
                        download: (*down / max_scale).clamp(0.0, 1.0) as f32,
                        upload: (*up / max_scale).clamp(0.0, 1.0) as f32,
                    })
                    .collect::<Vec<_>>();

                let weak = worker_weak.clone();
                let preview_weak = worker_preview_weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(window) = weak.upgrade() else {
                        return;
                    };
                    let download_text = format_speed(download);
                    let upload_text = format_speed(upload);
                    window.set_download_speed(download_text.as_str().into());
                    window.set_upload_speed(upload_text.as_str().into());
                    if let Some(preview) = preview_weak.upgrade() {
                        preview.set_download_speed(download_text.into());
                        preview.set_upload_speed(upload_text.into());
                    }
                    if window.get_page() != 0 || !window.window().is_visible() {
                        return;
                    }
                    let total = download + upload;
                    let total_display = metrics::speed(total);
                    window.set_total_speed(total_display.value.into());
                    window.set_total_speed_unit(total_display.unit.into());

                    let speed_history = window.get_speed_history();
                    for (index, sample) in speed_models.into_iter().enumerate() {
                        speed_history.set_row_data(index, sample);
                    }
                    window.set_graph_max_speed(format_speed(max_scale).into());
                    window.set_graph_mid_speed(format_speed(max_scale / 2.0).into());

                    window.set_total_level((total / max_scale).clamp(0.0, 1.0) as f32);
                    window.set_download_level((download / max_scale).min(1.0) as f32);
                    window.set_upload_level((upload / max_scale).min(1.0) as f32);
                });
            }
        });

        network_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_secs(1),
            move || {
                let Some(window) = weak.upgrade() else {
                    return;
                };
                let interval = sampling_preferences.borrow().network_interval_secs.max(1);
                let elapsed_ticks = sampling_ticks_for_timer.get() + 1;
                if elapsed_ticks < interval {
                    sampling_ticks_for_timer.set(elapsed_ticks);
                    return;
                }
                sampling_ticks_for_timer.set(0);
                let index = window.get_adapter_index().max(0) as usize;
                let interface_index = adapters
                    .lock()
                    .ok()
                    .and_then(|items| items.get(index).map(|item| item.interface_index));
                if let Some(interface_index) = interface_index {
                    let _ = sample_sender.try_send(Some(interface_index));
                } else {
                    let _ = sample_sender.try_send(None);
                    window.set_download_speed(format_speed(0.0).into());
                    window.set_upload_speed(format_speed(0.0).into());
                    window.set_total_speed("0".into());
                    window.set_total_speed_unit("Kbps".into());
                    window.set_total_level(0.0);
                    window.set_download_level(0.0);
                    window.set_upload_level(0.0);
                    if let Some(preview) = preview_weak.upgrade() {
                        preview.set_download_speed(window.get_download_speed());
                        preview.set_upload_speed(window.get_upload_speed());
                    }
                }
            },
        );
    }

    #[cfg(windows)]
    let mut tray_state = tray::create(&providers.borrow(), &language.borrow()).ok();
    #[cfg(windows)]
    window.set_tray_available(tray_state.is_some());

    #[cfg(windows)]
    let tray_timer = slint::Timer::default();
    #[cfg(windows)]
    {
        use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent, menu::MenuEvent};

        let app_weak = window.as_weak();
        let preview_weak = tray_preview.as_weak();
        let tray_providers = providers.clone();
        let tray_adapters = adapters.clone();
        let tray_language = language.clone();
        let tray_filter = provider_filter.clone();
        let mut menu_revision = window.get_tray_menu_revision();
        let last_active_menu_state = Rc::new(RefCell::new((
            SharedString::default(),
            SharedString::default(),
            SharedString::default(),
        )));
        let last_active_for_timer = last_active_menu_state.clone();
        let mut hover_dismissal = hover::HoverDismissal::default();
        let mut icon_bounds = None::<(f64, f64, f64, f64)>;
        tray_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(50),
            move || {
                if let Some(app) = app_weak.upgrade()
                    && menu_revision != app.get_tray_menu_revision()
                {
                    menu_revision = app.get_tray_menu_revision();
                    if let Some(state) = tray_state.as_mut() {
                        if let Err(error) =
                            state.refresh(&tray_providers.borrow(), &tray_language.borrow())
                        {
                            show_message(&app, error, true);
                        }
                        *last_active_for_timer.borrow_mut() = Default::default();
                    }
                }
                let Some(state) = tray_state.as_ref() else {
                    return;
                };
                let tray_actions = &state.dns_actions;
                let tray_items = &state.dns_items;
                let provider_menus = &state.provider_menus;
                if let Some(app) = app_weak.upgrade() {
                    let active_id = app.get_active_provider_id();
                    let active_title = app.get_active_provider();
                    let current_language = app.get_language();
                    let state = (
                        active_id.clone(),
                        active_title.clone(),
                        current_language.clone(),
                    );
                    if *last_active_for_timer.borrow() != state {
                        *last_active_for_timer.borrow_mut() = state;
                        for (provider_id, (provider_name, provider_menu)) in provider_menus.iter() {
                            if active_id != "system" && provider_id == active_id.as_str() {
                                provider_menu.set_text("✓ ".to_owned() + provider_name);
                            } else {
                                provider_menu.set_text(provider_name);
                            }
                        }
                        for (item_id, (provider_id, profile_index)) in tray_actions.iter() {
                            let checked = if active_id == "system"
                                || provider_id != active_id.as_str()
                            {
                                false
                            } else {
                                tray_providers
                                    .borrow()
                                    .iter()
                                    .find(|provider| provider.id == *provider_id)
                                    .map(|provider| {
                                        let profiles = provider.get_profiles();
                                        if profiles.len() <= 1 {
                                            true
                                        } else {
                                            profiles.get(*profile_index).is_some_and(|profile| {
                                                let profile_name = if current_language == "id" {
                                                    &profile.name_id
                                                } else {
                                                    &profile.name_en
                                                };
                                                active_title.ends_with(profile_name)
                                            })
                                        }
                                    })
                                    .unwrap_or(false)
                            };
                            if let Some(item) = tray_items.get(item_id) {
                                item.set_checked(checked);
                            }
                        }
                    }
                }

                while let Ok(event) = MenuEvent::receiver().try_recv() {
                    let id = event.id.0;
                    if id == tray::OPEN_ID {
                        if let Some(app) = app_weak.upgrade() {
                            app.window().set_minimized(false);
                            let _ = app.show();
                        }
                        continue;
                    }
                    if id == tray::CLOSE_ID {
                        if let Some(app) = app_weak.upgrade() {
                            if app.get_busy() {
                                let _ = app.show();
                                app.invoke_window_close();
                            } else {
                                let _ = slint::quit_event_loop();
                            }
                        }
                        continue;
                    }
                    let Some((provider_id, profile_index)) = tray_actions.get(&id).cloned() else {
                        continue;
                    };
                    if app_weak
                        .upgrade()
                        .is_none_or(|app| app.get_busy() || app.get_refreshing())
                    {
                        continue;
                    }
                    let provider = tray_providers
                        .borrow()
                        .iter()
                        .find(|provider| provider.id == provider_id)
                        .cloned();
                    let adapter = app_weak.upgrade().and_then(|app| {
                        let index = app.get_adapter_index().max(0) as usize;
                        tray_adapters
                            .lock()
                            .ok()
                            .and_then(|items| items.get(index).cloned())
                    });
                    let (Some(provider), Some(adapter)) = (provider, adapter) else {
                        if let Some(app) = app_weak.upgrade() {
                            let _ = app.show();
                            show_message(&app, "Select an active network adapter first.", true);
                        }
                        continue;
                    };
                    let profiles = provider.get_profiles();
                    let Some(profile) = profiles.get(profile_index) else {
                        continue;
                    };
                    let addresses = [profile.ipv4_primary.clone(), profile.ipv4_secondary.clone()]
                        .into_iter()
                        .filter(|address| !address.is_empty())
                        .collect::<Vec<_>>();
                    if addresses.is_empty() {
                        continue;
                    }
                    let lang = tray_language.borrow().clone();
                    let profile_name = if lang == "id" {
                        profile.name_id.clone()
                    } else {
                        profile.name_en.clone()
                    };
                    let active_title = if profiles.len() > 1 {
                        provider.name.clone() + " - " + &profile_name
                    } else {
                        provider.name.clone()
                    };
                    let template = doh_template(&provider.id, &profile.id);
                    let providers_snapshot = tray_providers.borrow().clone();
                    let (query, category) = tray_filter.borrow().clone();
                    if let Some(app) = app_weak.upgrade() {
                        app.set_busy(true);
                    }
                    let weak = app_weak.clone();
                    let preview = preview_weak.clone();
                    let active_provider_id = provider.id.clone();
                    let adapter_state = tray_adapters.clone();
                    std::thread::spawn(move || {
                        let result = system::apply_dns(
                            &adapter.name,
                            &addresses,
                            (!template.is_empty()).then_some(template),
                        );
                        let _ = slint::invoke_from_event_loop(move || {
                            let Some(app) = weak.upgrade() else {
                                return;
                            };
                            app.set_busy(false);
                            match result {
                                Ok(()) => {
                                    if let Ok(mut items) = adapter_state.lock()
                                        && let Some(item) =
                                            items.iter_mut().find(|item| item.name == adapter.name)
                                    {
                                        item.dns = addresses.join(", ");
                                        item.dns_automatic = false;
                                        item.dns_encrypted = !template.is_empty();
                                    }
                                    app.set_active_provider_id(active_provider_id.into());
                                    app.set_active_provider(active_title.clone().into());
                                    app.set_current_dns(addresses.join(", ").into());
                                    app.invoke_check_connection();
                                    set_filtered_provider_model(
                                        &app,
                                        &providers_snapshot,
                                        &lang,
                                        &addresses.join(", "),
                                        &query,
                                        &category,
                                    );
                                    if let Some(preview) = preview.upgrade() {
                                        preview.set_active_provider(active_title.into());
                                    }
                                    show_message(
                                        &app,
                                        if lang == "id" {
                                            "DNS berhasil diterapkan dari tray."
                                        } else {
                                            "DNS applied from the tray."
                                        },
                                        false,
                                    );
                                }
                                Err(message) => {
                                    let _ = app.show();
                                    show_message(&app, message, true);
                                }
                            }
                        });
                    });
                }

                while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                    match event {
                        TrayIconEvent::Enter { rect, .. } | TrayIconEvent::Move { rect, .. } => {
                            icon_bounds = Some((
                                rect.position.x,
                                rect.position.y,
                                rect.position.x + rect.size.width as f64,
                                rect.position.y + rect.size.height as f64,
                            ));
                            hover_dismissal.reset();
                            let (Some(app), Some(preview)) =
                                (app_weak.upgrade(), preview_weak.upgrade())
                            else {
                                continue;
                            };
                            preview.set_theme_mode(app.get_theme_mode());
                            preview.set_language(app.get_language());
                            preview.set_system_dark(app.get_system_dark());
                            preview.set_download_speed(app.get_download_speed());
                            preview.set_upload_speed(app.get_upload_speed());
                            preview.set_active_provider(app.get_active_provider());
                            preview.set_connection_state(app.get_connection_state());
                            preview.set_latency(app.get_latency());
                            if !preview.window().is_visible()
                                && native_windows::show_preview(&preview).is_ok()
                            {
                                let size = preview.window().size();
                                let right = rect.position.x + rect.size.width as f64;
                                let mut x = (right - size.width as f64).round() as i32;
                                let mut y =
                                    (rect.position.y - size.height as f64 - 8.0).round() as i32;
                                use slint::winit_030::WinitWindowAccessor;
                                preview.window().with_winit_window(|native| {
                                    for monitor in native.available_monitors() {
                                        let origin = monitor.position();
                                        let bounds = monitor.size();
                                        let monitor_right = origin.x + bounds.width as i32;
                                        let monitor_bottom = origin.y + bounds.height as i32;
                                        if rect.position.x >= origin.x as f64
                                            && rect.position.x < monitor_right as f64
                                            && rect.position.y >= origin.y as f64
                                            && rect.position.y < monitor_bottom as f64
                                        {
                                            if y < origin.y {
                                                y = (rect.position.y
                                                    + rect.size.height as f64
                                                    + 8.0)
                                                    .round()
                                                    as i32;
                                            }
                                            x = x.clamp(
                                                origin.x,
                                                (monitor_right - size.width as i32).max(origin.x),
                                            );
                                            y = y.clamp(
                                                origin.y,
                                                (monitor_bottom - size.height as i32).max(origin.y),
                                            );
                                            break;
                                        }
                                    }
                                });
                                preview
                                    .window()
                                    .set_position(slint::PhysicalPosition::new(x, y));
                            }
                        }
                        TrayIconEvent::Leave { .. } => {}
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            hover_dismissal.reset();
                            if let Some(preview) = preview_weak.upgrade() {
                                let _ = preview.hide();
                            }
                            if let Some(app) = app_weak.upgrade() {
                                app.window().set_minimized(false);
                                let _ = app.show();
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(preview) = preview_weak.upgrade()
                    && preview.window().is_visible()
                {
                    if let Some(app) = app_weak.upgrade() {
                        preview.set_theme_mode(app.get_theme_mode());
                        preview.set_language(app.get_language());
                        preview.set_system_dark(app.get_system_dark());
                        preview.set_active_provider(app.get_active_provider());
                        preview.set_connection_state(app.get_connection_state());
                        preview.set_latency(app.get_latency());
                    }
                    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                    let mut point = windows::Win32::Foundation::POINT::default();
                    // SAFETY: point is a valid writable POINT. Polling real cursor position
                    // avoids missed tray Leave events and stale UI hover flags.
                    let cursor_known = unsafe { GetCursorPos(&mut point) != 0 };
                    let over_icon = cursor_known
                        && icon_bounds.is_some_and(|(left, top, right, bottom)| {
                            point.x as f64 >= left
                                && (point.x as f64) < right
                                && point.y as f64 >= top
                                && (point.y as f64) < bottom
                        });
                    let mut over_preview = false;
                    if cursor_known
                        && let Ok(handle) = preview.window().window_handle().window_handle()
                        && let RawWindowHandle::Win32(win32) = handle.as_raw()
                    {
                        let mut bounds = windows::Win32::Foundation::RECT::default();
                        // SAFETY: Slint owns this HWND, bounds is writable.
                        if unsafe { GetWindowRect(win32.hwnd.get() as _, &mut bounds) } != 0 {
                            over_preview = point.x >= bounds.left
                                && point.x < bounds.right
                                && point.y >= bounds.top
                                && point.y < bounds.bottom;
                        }
                    }
                    if hover_dismissal.should_hide(Instant::now(), over_icon, over_preview) {
                        let _ = preview.hide();
                        hover_dismissal.reset();
                    }
                }
            },
        );
    }

    window.invoke_refresh();
    if !std::env::args().any(|arg| arg == "--background") || !window.get_tray_available() {
        window.show()?;
    }
    slint::run_event_loop_until_quit()
}
