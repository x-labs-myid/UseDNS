mod models;
mod storage;
mod system;

use models::DnsProvider;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
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

fn row(provider: &DnsProvider, language: &str, current_dns: &str) -> ProviderRow {
    let addresses = [
        provider.ipv4_primary.as_str(),
        provider.ipv4_secondary.as_str(),
        provider.ipv6_primary.as_str(),
        provider.ipv6_secondary.as_str(),
    ];
    let is_active = addresses
        .iter()
        .filter(|value| !value.is_empty())
        .any(|value| current_dns.contains(*value));

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

fn format_pair(primary: &str, secondary: &str) -> String {
    match (primary.is_empty(), secondary.is_empty()) {
        (true, _) => "—".into(),
        (false, true) => primary.into(),
        (false, false) => format!("{primary}, {secondary}"),
    }
}

fn format_speed(mbps: f64, unit: &str) -> String {
    if unit == "kbps" {
        let kbps = mbps * 1000.0;
        if kbps < 10.0 {
            format!("{kbps:.1} Kbps")
        } else {
            format!("{kbps:.0} Kbps")
        }
    } else if mbps < 1.0 {
        format!("{mbps:.2} Mbps")
    } else {
        format!("{mbps:.1} Mbps")
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
        .map(|provider| row(provider, language, current_dns))
        .collect::<Vec<_>>();
    window.set_providers(ModelRc::from(Rc::new(VecModel::from(rows))));
}

fn show_message(window: &AppWindow, message: impl Into<SharedString>, error: bool) {
    window.set_toast_message(message.into());
    window.set_toast_error(error);
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
    fn ReleaseCapture() -> i32;
    fn PostMessageW(hwnd: *mut std::ffi::c_void, msg: u32, wparam: usize, lparam: isize) -> i32;
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
}

fn main() -> Result<(), slint::PlatformError> {
    let window = AppWindow::new()?;

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
        let weak = window.as_weak();
        window.on_window_drag(move || {
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
                    // SAFETY: GetActiveWindow and GetForegroundWindow are standard Win32 queries with no preconditions.
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
                    // SAFETY: target_hwnd was checked non-null and refers to a valid top-level window.
                    // ReleaseCapture releases any pointer capture, and WM_NCLBUTTONDOWN initiates window moving.
                    unsafe {
                        ReleaseCapture();
                        PostMessageW(
                            target_hwnd,
                            0x00A1, /* WM_NCLBUTTONDOWN */
                            2,      /* HTCAPTION */
                            0,
                        );
                    }
                }
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

    {
        window.on_window_close(move || {
            std::process::exit(0);
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

    let mut initial = models::default_providers();
    initial.append(&mut settings.custom_providers);
    let providers = Rc::new(RefCell::new(initial));
    let language = Rc::new(RefCell::new(settings.language));
    let theme = Rc::new(RefCell::new(settings.theme));
    let speed_unit = Rc::new(RefCell::new(settings.speed_unit));
    let adapters = Arc::new(Mutex::new(Vec::<system::AdapterInfo>::new()));

    window.set_system_dark(system::prefers_dark_mode());
    window.set_language(language.borrow().as_str().into());
    window.set_theme_mode(theme.borrow().as_str().into());
    window.set_speed_unit(speed_unit.borrow().as_str().into());
    set_provider_model(&window, &providers.borrow(), &language.borrow(), "");

    {
        let weak = window.as_weak();
        let adapters = adapters.clone();
        window.on_refresh(move || {
            let weak = weak.clone();
            let adapters = adapters.clone();
            if let Some(window) = weak.upgrade() {
                window.set_busy(true);
                window.set_toast_message("".into());
            }
            std::thread::spawn(move || {
                let result = system::active_adapters();
                let _ = slint::invoke_from_event_loop(move || {
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
                                let dns = selected.dns.clone();
                                let weak_check = weak.clone();
                                std::thread::spawn(move || {
                                    let result = system::check_connection(&dns);
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(window) = weak_check.upgrade() {
                                            window.set_busy(false);
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
                                window.set_busy(false);
                            }
                            if let Ok(mut cached) = adapters.lock() {
                                *cached = found;
                            }
                        }
                        Err(message) => {
                            window.set_busy(false);
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
        let weak = window.as_weak();
        window.on_language_changed(move |new_language| {
            let value = if new_language.as_str() == "id" {
                "id"
            } else {
                "en"
            };
            *language.borrow_mut() = value.into();
            if let Some(window) = weak.upgrade() {
                set_provider_model(
                    &window,
                    &providers.borrow(),
                    value,
                    &window.get_current_dns(),
                );
            }
            let settings = storage::Settings {
                language: value.into(),
                theme: theme.borrow().clone(),
                speed_unit: speed_unit.borrow().clone(),
                custom_providers: providers
                    .borrow()
                    .iter()
                    .filter(|provider| provider.custom)
                    .cloned()
                    .collect(),
            };
            let _ = storage::save(&settings);
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let theme = theme.clone();
        let speed_unit = speed_unit.clone();
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
            let settings = storage::Settings {
                language: language.borrow().clone(),
                theme: value.into(),
                speed_unit: speed_unit.borrow().clone(),
                custom_providers: providers
                    .borrow()
                    .iter()
                    .filter(|provider| provider.custom)
                    .cloned()
                    .collect(),
            };
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
        let weak = window.as_weak();
        window.on_speed_unit_changed(move |new_unit| {
            let value = match new_unit.as_str() {
                "mbps" => "mbps",
                _ => "kbps",
            };
            *speed_unit.borrow_mut() = value.into();
            let settings = storage::Settings {
                language: language.borrow().clone(),
                theme: theme.borrow().clone(),
                speed_unit: value.into(),
                custom_providers: providers
                    .borrow()
                    .iter()
                    .filter(|provider| provider.custom)
                    .cloned()
                    .collect(),
            };
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
        let weak = window.as_weak();
        window.on_filter_providers(move |query, category| {
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
                    }
                })
                .collect::<Vec<_>>();

            if let Some(window) = weak.upgrade() {
                window.set_popup_provider_id(provider.id.into());
                window.set_popup_provider_name(provider.name.into());
                window.set_popup_profiles(ModelRc::from(Rc::new(VecModel::from(profile_rows))));
                window.set_selected_profile_index(0);
                window.set_profile_popup_open(true);
            }
        });
    }

    {
        let providers = providers.clone();
        let adapters = adapters.clone();
        let language = language.clone();
        let weak = window.as_weak();
        window.on_confirm_apply_profile(move |id, profile_index, adapter_index, mode| {
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
                format!("{} - {}", provider.name, profile_display_name)
            } else {
                provider.name.clone()
            };

            std::thread::spawn(move || {
                let result = system::apply_dns(&adapter.name, &addresses);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
                        match result {
                            Ok(()) => {
                                window.set_active_provider(active_title.into());
                                window.set_current_dns(addresses.join(", ").into());
                                show_message(
                                    &window,
                                    if lang == "id" {
                                        "DNS berhasil diterapkan."
                                    } else {
                                        "DNS applied successfully."
                                    },
                                    false,
                                );
                                window.invoke_refresh();
                            }
                            Err(message) => show_message(&window, message, true),
                        }
                    }
                });
            });
        });
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
            let language_value = language.borrow().clone();
            std::thread::spawn(move || {
                let result = system::apply_dns(&adapter.name, &addresses);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
                        match result {
                            Ok(()) => {
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
                                window.invoke_refresh();
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
                                window.set_active_provider("System / DHCP".into());
                                show_message(
                                    &window,
                                    if language == "id" {
                                        "DNS otomatis berhasil dipulihkan."
                                    } else {
                                        "Automatic DNS restored."
                                    },
                                    false,
                                );
                                window.invoke_refresh();
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
                window.set_busy(true);
                window.set_toast_message("".into());
            }
            let weak = weak.clone();
            let language = language.borrow().clone();
            std::thread::spawn(move || {
                let result = system::check_connection(&address);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
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
        let weak = window.as_weak();
        window.on_save_provider(move |id, name, v4a, v4b, v6a, v6b, summary, purpose| {
            let generated_id = if id.is_empty() {
                format!(
                    "custom-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                )
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
            let settings = storage::Settings {
                language: language.borrow().clone(),
                theme: theme.borrow().clone(),
                speed_unit: speed_unit.borrow().clone(),
                custom_providers: list.iter().filter(|item| item.custom).cloned().collect(),
            };
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
        let weak = window.as_weak();
        window.on_delete_provider(move |id| {
            providers
                .borrow_mut()
                .retain(|provider| !provider.custom || provider.id != id.as_str());
            let list = providers.borrow();
            let settings = storage::Settings {
                language: language.borrow().clone(),
                theme: theme.borrow().clone(),
                speed_unit: speed_unit.borrow().clone(),
                custom_providers: list.iter().filter(|item| item.custom).cloned().collect(),
            };
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
        let adapters = adapters.clone();
        let speed_unit = speed_unit.clone();
        let previous = Arc::new(Mutex::new(None::<(String, u64, u64, Instant)>));
        let history_samples = Arc::new(Mutex::new(vec![(0.0f64, 0.0f64); 24]));
        let sampling = Arc::new(AtomicBool::new(false));

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

        let history_samples_clone = history_samples.clone();
        network_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_secs(1),
            move || {
                if sampling.swap(true, Ordering::AcqRel) {
                    return;
                }
                let Some(window) = weak.upgrade() else {
                    sampling.store(false, Ordering::Release);
                    return;
                };
                let index = window.get_adapter_index().max(0) as usize;
                let adapter = adapters
                    .lock()
                    .ok()
                    .and_then(|items| items.get(index).map(|item| item.name.clone()));
                let Some(adapter) = adapter else {
                    sampling.store(false, Ordering::Release);
                    return;
                };
                let weak = weak.clone();
                let previous = previous.clone();
                let history_samples = history_samples_clone.clone();
                let sampling = sampling.clone();
                let current_unit = speed_unit.borrow().clone();
                std::thread::spawn(move || {
                    let result = system::network_counters(&adapter).map(|(received, sent)| {
                        let now = Instant::now();
                        let mut history = match previous.lock() {
                            Ok(guard) => guard,
                            Err(poisoned) => poisoned.into_inner(),
                        };
                        let speeds = history
                            .as_ref()
                            .filter(|(name, old_received, old_sent, _)| {
                                name == &adapter && received >= *old_received && sent >= *old_sent
                            })
                            .map(|(_, old_received, old_sent, sampled_at)| {
                                let seconds =
                                    now.duration_since(*sampled_at).as_secs_f64().max(0.001);
                                (
                                    (received - old_received) as f64 * 8.0 / seconds / 1_000_000.0,
                                    (sent - old_sent) as f64 * 8.0 / seconds / 1_000_000.0,
                                )
                            })
                            .unwrap_or((0.0, 0.0));
                        *history = Some((adapter, received, sent, now));
                        speeds
                    });
                    sampling.store(false, Ordering::Release);
                    let _ = slint::invoke_from_event_loop(move || {
                        if let (Some(window), Ok((download, upload))) = (weak.upgrade(), result) {
                            let total = download + upload;
                            window.set_download_speed(format_speed(download, &current_unit).into());
                            window.set_upload_speed(format_speed(upload, &current_unit).into());
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

                            // Update scrolling 24-second history
                            let mut samples = match history_samples.lock() {
                                Ok(guard) => guard,
                                Err(poisoned) => poisoned.into_inner(),
                            };
                            if !samples.is_empty() {
                                samples.remove(0);
                            }
                            samples.push((download, upload));

                            // Find peak speed across the history window (minimum 0.5 Mbps)
                            let peak_mbps = samples
                                .iter()
                                .map(|(d, u)| d.max(*u))
                                .fold(0.5f64, |a, b| a.max(b));
                            let max_scale = peak_mbps * 1.15;

                            let speed_models: Vec<SpeedSample> = samples
                                .iter()
                                .map(|(d, u)| SpeedSample {
                                    download: (*d / max_scale).clamp(0.0, 1.0) as f32,
                                    upload: (*u / max_scale).clamp(0.0, 1.0) as f32,
                                })
                                .collect();

                            window.set_speed_history(ModelRc::from(Rc::new(VecModel::from(
                                speed_models,
                            ))));
                            window
                                .set_graph_max_speed(format_speed(max_scale, &current_unit).into());
                            window.set_graph_mid_speed(
                                format_speed(max_scale / 2.0, &current_unit).into(),
                            );

                            // Arc meter level based on dynamic peak or unit scale
                            let total_level = if current_unit == "kbps" {
                                let kbps = total * 1000.0;
                                (kbps / 10000.0).min(1.0) as f32
                            } else {
                                (total / max_scale).min(1.0) as f32
                            };
                            window.set_total_level(
                                total_level.max((total / max_scale).min(1.0) as f32),
                            );
                            window.set_download_level((download / max_scale).min(1.0) as f32);
                            window.set_upload_level((upload / max_scale).min(1.0) as f32);
                        }
                    });
                });
            },
        );
    }

    window.invoke_refresh();
    window.run()
}
