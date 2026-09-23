#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod storage;
mod system;
#[cfg(windows)]
mod tray;

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

fn format_fixed(value: f64, decimals: u32) -> String {
    let factor = 10_i64.pow(decimals) as f64;
    let scaled = (value * factor).round() as i64;
    if decimals == 0 {
        return scaled.to_string();
    }

    let whole = scaled / factor as i64;
    let fraction = (scaled % factor as i64).abs();
    let mut fraction_text = fraction.to_string();
    while fraction_text.len() < decimals as usize {
        fraction_text.insert(0, '0');
    }
    whole.to_string() + "." + &fraction_text
}

fn format_speed(mbps: f64, unit: &str) -> String {
    if unit == "kbps" {
        let kbps = mbps * 1000.0;
        let decimals = if kbps < 10.0 { 1 } else { 0 };
        format_fixed(kbps, decimals) + " Kbps"
    } else {
        let decimals = if mbps < 1.0 { 2 } else { 1 };
        format_fixed(mbps, decimals) + " Mbps"
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
    network_interval_secs: u32,
    show_speed: bool,
    show_dns: bool,
    show_status: bool,
    show_latency: bool,
}

fn settings_snapshot(
    language: &str,
    theme: &str,
    speed_unit: &str,
    providers: &[DnsProvider],
    tray: TrayPreferences,
) -> storage::Settings {
    storage::Settings {
        language: language.into(),
        theme: theme.into(),
        speed_unit: speed_unit.into(),
        network_interval_secs: tray.network_interval_secs,
        tray_show_speed: tray.show_speed,
        tray_show_dns: tray.show_dns,
        tray_show_status: tray.show_status,
        tray_show_latency: tray.show_latency,
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
    fn ShowWindow(hwnd: *mut std::ffi::c_void, ncmdshow: i32) -> i32;
    fn DwmExtendFrameIntoClientArea(hwnd: *mut std::ffi::c_void, pmargins: *const Margins) -> i32;
    fn DwmSetWindowAttribute(
        hwnd: *mut std::ffi::c_void,
        dw_attribute: u32,
        pv_attribute: *const std::ffi::c_void,
        cb_attribute: u32,
    ) -> i32;
    fn GetForegroundWindow() -> *mut std::ffi::c_void;
    fn GetActiveWindow() -> *mut std::ffi::c_void;
    fn GetWindowLongPtrW(hwnd: *mut std::ffi::c_void, index: i32) -> isize;
    fn SetWindowLongPtrW(hwnd: *mut std::ffi::c_void, index: i32, value: isize) -> isize;
}

fn main() -> Result<(), slint::PlatformError> {
    if let Some(exit_code) = system::run_dns_helper_if_requested() {
        std::process::exit(exit_code);
    }

    let window = AppWindow::new()?;
    let tray_preview = TrayPreview::new()?;

    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = tray_preview.window().window_handle().window_handle()
            && let RawWindowHandle::Win32(win32) = handle.as_raw()
        {
            const GWL_EXSTYLE: i32 = -20;
            const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
            const WS_EX_APPWINDOW: isize = 0x0004_0000;
            const WS_EX_NOACTIVATE: isize = 0x0800_0000;
            let preview_hwnd = win32.hwnd.get() as *mut std::ffi::c_void;
            // SAFETY: preview_hwnd is owned by Slint and valid for the component lifetime.
            unsafe {
                let style = GetWindowLongPtrW(preview_hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(
                    preview_hwnd,
                    GWL_EXSTYLE,
                    (style | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) & !WS_EX_APPWINDOW,
                );
            }
        }
    }

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

                if target_hwnd.is_null() {
                    if let Some(w) = weak.upgrade() {
                        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                        if let Ok(handle) = w.window().window_handle().window_handle() {
                            if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                                target_hwnd = win32.hwnd.get() as _;
                            }
                        }
                    }
                }

                if target_hwnd.is_null() {
                    // SAFETY: GetActiveWindow and GetForegroundWindow are safe queries with no preconditions.
                    unsafe {
                        let active = GetActiveWindow();
                        if !active.is_null() {
                            target_hwnd = active;
                        } else {
                            target_hwnd = GetForegroundWindow();
                        }
                    }
                }

                if !target_hwnd.is_null() {
                    // SAFETY: target_hwnd is non-null and SW_MINIMIZE is a valid command for ShowWindow.
                    unsafe {
                        ShowWindow(target_hwnd, 6 /* SW_MINIMIZE */);
                    }
                }
            }
            #[cfg(not(windows))]
            if let Some(w) = weak.upgrade() {
                w.window().set_minimized(true);
            }
        });
    }

    window
        .window()
        .on_close_requested(|| slint::CloseRequestResponse::HideWindow);

    {
        let weak = window.as_weak();
        window.on_window_close(move || {
            if let Some(window) = weak.upgrade() {
                let _ = window.hide();
            }
        });
    }

    let mut settings = storage::load();
    if settings.language != "id" {
        settings.language = "en".into();
    }
    if !matches!(settings.theme.as_str(), "system" | "light" | "dark") {
        settings.theme = "system".into();
    }
    if !matches!(settings.speed_unit.as_str(), "kbps" | "mbps") {
        settings.speed_unit = "kbps".into();
    }
    if !matches!(settings.network_interval_secs, 1 | 2 | 5) {
        settings.network_interval_secs = 1;
    }
    let tray_preferences = Rc::new(RefCell::new(TrayPreferences {
        network_interval_secs: settings.network_interval_secs,
        show_speed: settings.tray_show_speed,
        show_dns: settings.tray_show_dns,
        show_status: settings.tray_show_status,
        show_latency: settings.tray_show_latency,
    }));

    let mut initial = models::default_providers();
    initial.append(&mut settings.custom_providers);
    let providers = Rc::new(RefCell::new(initial));
    let language = Rc::new(RefCell::new(settings.language));
    let theme = Rc::new(RefCell::new(settings.theme));
    let speed_unit = Rc::new(RefCell::new(settings.speed_unit));
    let adapters = Arc::new(Mutex::new(Vec::<system::AdapterInfo>::new()));
    let provider_filter = Rc::new(RefCell::new((String::new(), String::from("all"))));

    window.set_system_dark(system::prefers_dark_mode());
    window.set_language(language.borrow().as_str().into());
    window.set_theme_mode(theme.borrow().as_str().into());
    window.set_speed_unit(speed_unit.borrow().as_str().into());
    window.set_network_interval_secs(settings.network_interval_secs as i32);
    window.set_tray_show_speed(settings.tray_show_speed);
    window.set_tray_show_dns(settings.tray_show_dns);
    window.set_tray_show_status(settings.tray_show_status);
    window.set_tray_show_latency(settings.tray_show_latency);
    tray_preview.set_theme_mode(theme.borrow().as_str().into());
    tray_preview.set_system_dark(window.get_system_dark());
    tray_preview.set_show_speed(settings.tray_show_speed);
    tray_preview.set_show_dns(settings.tray_show_dns);
    tray_preview.set_show_status(settings.tray_show_status);
    tray_preview.set_show_latency(settings.tray_show_latency);
    set_provider_model(&window, &providers.borrow(), &language.borrow(), "");

    {
        let weak = window.as_weak();
        let adapters = adapters.clone();
        let providers = providers.clone();
        let language = language.clone();
        let provider_filter = provider_filter.clone();
        let refreshing = Arc::new(AtomicBool::new(false));
        window.on_refresh(move || {
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
                            let index = window.get_adapter_index().max(0) as usize;
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
                                let dns = selected.dns.clone();
                                let weak_check = weak.clone();
                                std::thread::spawn(move || {
                                    let result = system::check_connection(&dns);
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(window) = weak_check.upgrade() {
                                            match result {
                                                Ok(ms) => {
                                                    window.set_latency(format!("{ms} ms").into());
                                                    let state = if ms < 100 {
                                                        "Stable"
                                                    } else if ms < 250 {
                                                        "Fair"
                                                    } else {
                                                        "Slow"
                                                    };
                                                    window.set_connection_state(state.into());
                                                }
                                                Err(message) => {
                                                    window.set_latency("—".into());
                                                    window.set_connection_state("Unstable".into());
                                                    show_message(&window, message, true);
                                                }
                                            }
                                        }
                                    });
                                });
                            } else {
                                window.set_refreshing(false);
                            }
                            if let Ok(mut cached) = adapters.lock() {
                                *cached = found;
                            }
                        }
                        Err(message) => {
                            window.set_refreshing(false);
                            show_message(&window, message, true);
                        }
                    }
                });
            });
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let speed_unit = speed_unit.clone();
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
                &speed_unit.borrow(),
                &providers.borrow(),
                *tray_preferences.borrow(),
            );
            let _ = storage::save(&settings);
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let speed_unit = speed_unit.clone();
        let tray_preferences = tray_preferences.clone();
        let weak = window.as_weak();
        window.on_theme_changed(move |new_theme| {
            let value = match new_theme.as_str() {
                "light" => "light",
                "dark" => "dark",
                _ => "system",
            };
            *theme.borrow_mut() = value.into();
            if value == "system" {
                if let Some(window) = weak.upgrade() {
                    window.set_system_dark(system::prefers_dark_mode());
                }
            }
            let settings = settings_snapshot(
                &language.borrow(),
                value,
                &speed_unit.borrow(),
                &providers.borrow(),
                *tray_preferences.borrow(),
            );
            if let Err(error) = storage::save(&settings) {
                if let Some(window) = weak.upgrade() {
                    show_message(&window, format!("Could not save settings: {error}"), true);
                }
            }
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let speed_unit = speed_unit.clone();
        let tray_preferences = tray_preferences.clone();
        let weak = window.as_weak();
        window.on_speed_unit_changed(move |new_unit| {
            let value = match new_unit.as_str() {
                "mbps" => "mbps",
                _ => "kbps",
            };
            *speed_unit.borrow_mut() = value.into();
            let settings = settings_snapshot(
                &language.borrow(),
                &theme.borrow(),
                value,
                &providers.borrow(),
                *tray_preferences.borrow(),
            );
            if let Err(error) = storage::save(&settings) {
                if let Some(window) = weak.upgrade() {
                    show_message(&window, format!("Could not save settings: {error}"), true);
                }
            }
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let speed_unit = speed_unit.clone();
        let tray_preferences = tray_preferences.clone();
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
                &speed_unit.borrow(),
                &providers.borrow(),
                *tray_preferences.borrow(),
            );
            let _ = storage::save(&settings);
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let speed_unit = speed_unit.clone();
        let tray_preferences = tray_preferences.clone();
        let preview_weak = tray_preview.as_weak();
        window.on_tray_preview_settings_changed(move |speed, dns, status, latency| {
            let preferences = TrayPreferences {
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
                &speed_unit.borrow(),
                &providers.borrow(),
                preferences,
            );
            let _ = storage::save(&settings);
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
            std::thread::spawn(move || {
                let result = system::apply_dns(&adapter.name, &addresses, None);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
                        match result {
                            Ok(()) => {
                                window.set_active_provider_id(provider_id.into());
                                window.set_active_provider(provider_name.clone().into());
                                window.set_current_dns(addresses.join(", ").into());
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
        let speed_unit = speed_unit.clone();
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
            let mut list = providers.borrow_mut();
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
                &speed_unit.borrow(),
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
                window.set_page(1);
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
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let speed_unit = speed_unit.clone();
        let tray_preferences = tray_preferences.clone();
        let weak = window.as_weak();
        window.on_delete_provider(move |id| {
            providers
                .borrow_mut()
                .retain(|provider| !provider.custom || provider.id != id.as_str());
            let list = providers.borrow();
            let settings = settings_snapshot(
                &language.borrow(),
                &theme.borrow(),
                &speed_unit.borrow(),
                &list,
                *tray_preferences.borrow(),
            );
            let result = storage::save(&settings);
            if let Some(window) = weak.upgrade() {
                set_provider_model(
                    &window,
                    &list,
                    &language.borrow(),
                    &window.get_current_dns(),
                );
                match result {
                    Ok(()) => show_message(
                        &window,
                        if *language.borrow() == "id" {
                            "DNS kustom dihapus."
                        } else {
                            "Custom DNS deleted."
                        },
                        false,
                    ),
                    Err(error) => {
                        show_message(&window, format!("Could not save settings: {error}"), true)
                    }
                }
            }
        });
    }

    let network_timer = slint::Timer::default();
    {
        let weak = window.as_weak();
        let preview_weak = tray_preview.as_weak();
        let adapters = adapters.clone();
        let speed_unit = speed_unit.clone();
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
        let initial_unit = speed_unit.borrow().clone();
        window.set_graph_max_speed(format_speed(1.0, &initial_unit).into());
        window.set_graph_mid_speed(format_speed(0.5, &initial_unit).into());

        let (sample_sender, sample_receiver) =
            std::sync::mpsc::sync_channel::<Option<(u32, String)>>(1);
        let worker_weak = weak.clone();
        let worker_preview_weak = preview_weak.clone();
        std::thread::spawn(move || {
            let mut previous = None::<(u32, u64, u64, Instant)>;
            let mut samples = vec![(0.0f64, 0.0f64); 24];

            while let Ok(request) = sample_receiver.recv() {
                let Some((interface_index, current_unit)) = request else {
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
                    let download_text = format_speed(download, &current_unit);
                    let upload_text = format_speed(upload, &current_unit);
                    window.set_download_speed(download_text.as_str().into());
                    window.set_upload_speed(upload_text.as_str().into());
                    if let Some(preview) = preview_weak.upgrade() {
                        preview.set_download_speed(download_text.into());
                        preview.set_upload_speed(upload_text.into());
                    }
                    if window.get_page() != 0 {
                        return;
                    }
                    let total = download + upload;
                    let total_display = if current_unit == "kbps" {
                        let kbps = total * 1000.0;
                        if kbps < 10.0 {
                            format!("{kbps:.1}")
                        } else {
                            format!("{kbps:.0}")
                        }
                    } else {
                        format!("{total:.1}")
                    };
                    window.set_total_speed(total_display.into());

                    let speed_history = window.get_speed_history();
                    for (index, sample) in speed_models.into_iter().enumerate() {
                        speed_history.set_row_data(index, sample);
                    }
                    window.set_graph_max_speed(format_speed(max_scale, &current_unit).into());
                    window.set_graph_mid_speed(format_speed(max_scale / 2.0, &current_unit).into());

                    let total_level = if current_unit == "kbps" {
                        (total * 1000.0 / 10000.0).min(1.0) as f32
                    } else {
                        (total / max_scale).min(1.0) as f32
                    };
                    window.set_total_level(total_level.max((total / max_scale).min(1.0) as f32));
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
                    let current_unit = speed_unit.borrow().clone();
                    let _ = sample_sender.try_send(Some((interface_index, current_unit)));
                }
            },
        );
    }

    #[cfg(windows)]
    let tray_state = tray::create(&providers.borrow(), &language.borrow()).ok();
    #[cfg(windows)]
    let tray_dns_actions = Rc::new(
        tray_state
            .as_ref()
            .map(|state| state.dns_actions.clone())
            .unwrap_or_default(),
    );
    #[cfg(windows)]
    let tray_dns_items = Rc::new(
        tray_state
            .as_ref()
            .map(|state| state.dns_items.clone())
            .unwrap_or_default(),
    );
    #[cfg(windows)]
    let tray_provider_menus = Rc::new(
        tray_state
            .as_ref()
            .map(|state| state.provider_menus.clone())
            .unwrap_or_default(),
    );
    #[cfg(windows)]
    let _tray_icon = tray_state.map(|state| state.icon);

    #[cfg(windows)]
    let tray_timer = slint::Timer::default();
    #[cfg(windows)]
    {
        use std::cell::Cell;
        use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent, menu::MenuEvent};

        let app_weak = window.as_weak();
        let preview_weak = tray_preview.as_weak();
        let tray_providers = providers.clone();
        let tray_adapters = adapters.clone();
        let tray_language = language.clone();
        let tray_filter = provider_filter.clone();
        let tray_actions = tray_dns_actions.clone();
        let tray_items = tray_dns_items.clone();
        let provider_menus = tray_provider_menus.clone();
        let last_active_menu_state =
            Rc::new(RefCell::new((String::new(), String::new(), String::new())));
        let last_active_for_timer = last_active_menu_state.clone();
        let hide_at = Rc::new(Cell::new(None::<Instant>));
        let hide_at_for_timer = hide_at.clone();
        tray_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(50),
            move || {
                if let Some(app) = app_weak.upgrade() {
                    let active_id = app.get_active_provider_id().to_string();
                    let active_title = app.get_active_provider().to_string();
                    let current_language = tray_language.borrow().clone();
                    let state = (
                        active_id.clone(),
                        active_title.clone(),
                        current_language.clone(),
                    );
                    if *last_active_for_timer.borrow() != state {
                        *last_active_for_timer.borrow_mut() = state;
                        for (provider_id, (provider_name, provider_menu)) in provider_menus.iter() {
                            if active_id != "system" && provider_id == &active_id {
                                provider_menu.set_text("✓ ".to_owned() + provider_name);
                            } else {
                                provider_menu.set_text(provider_name);
                            }
                        }
                        for (item_id, (provider_id, profile_index)) in tray_actions.iter() {
                            let checked = if active_id == "system" || provider_id != &active_id {
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
                        let _ = slint::quit_event_loop();
                        continue;
                    }
                    let Some((provider_id, profile_index)) = tray_actions.get(&id).cloned() else {
                        continue;
                    };
                    if app_weak.upgrade().is_some_and(|app| app.get_busy()) {
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
                                    app.set_active_provider_id(active_provider_id.into());
                                    app.set_active_provider(active_title.clone().into());
                                    app.set_current_dns(addresses.join(", ").into());
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
                            hide_at_for_timer.set(None);
                            let (Some(app), Some(preview)) =
                                (app_weak.upgrade(), preview_weak.upgrade())
                            else {
                                continue;
                            };
                            preview.set_theme_mode(app.get_theme_mode());
                            preview.set_system_dark(app.get_system_dark());
                            preview.set_download_speed(app.get_download_speed());
                            preview.set_upload_speed(app.get_upload_speed());
                            preview.set_active_provider(app.get_active_provider());
                            preview.set_connection_state(app.get_connection_state());
                            preview.set_latency(app.get_latency());
                            if preview.show().is_ok() {
                                let size = preview.window().size();
                                let right = rect.position.x + rect.size.width as f64;
                                let x = (right - size.width as f64).round() as i32;
                                let y = (rect.position.y - size.height as f64 - 8.0).round() as i32;
                                preview
                                    .window()
                                    .set_position(slint::PhysicalPosition::new(x, y));
                            }
                        }
                        TrayIconEvent::Leave { .. } => {
                            hide_at_for_timer
                                .set(Some(Instant::now() + Duration::from_millis(280)));
                        }
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            hide_at_for_timer.set(None);
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

                if hide_at_for_timer
                    .get()
                    .is_some_and(|deadline| Instant::now() >= deadline)
                {
                    hide_at_for_timer.set(None);
                    if let Some(preview) = preview_weak.upgrade() {
                        let _ = preview.hide();
                    }
                }
            },
        );
    }

    window.invoke_refresh();
    window.show()?;
    slint::run_event_loop_until_quit()
}
