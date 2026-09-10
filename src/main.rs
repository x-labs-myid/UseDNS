mod models;
mod storage;
mod system;

use models::DnsProvider;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

slint::include_modules!();

fn row(provider: &DnsProvider, language: &str, current_dns: &str) -> ProviderRow {
    let addresses = [
        provider.ipv4_primary.as_str(),
        provider.ipv4_secondary.as_str(),
        provider.ipv6_primary.as_str(),
        provider.ipv6_secondary.as_str(),
    ];
    ProviderRow {
        id: provider.id.clone().into(),
        name: provider.name.clone().into(),
        ipv4: format_pair(&provider.ipv4_primary, &provider.ipv4_secondary).into(),
        ipv6: format_pair(&provider.ipv6_primary, &provider.ipv6_secondary).into(),
        summary: if language == "id" {
            provider.summary_id.clone()
        } else {
            provider.summary_en.clone()
        }
        .into(),
        pros: if language == "id" {
            provider.pros_id.clone()
        } else {
            provider.pros_en.clone()
        }
        .into(),
        cons: if language == "id" {
            provider.cons_id.clone()
        } else {
            provider.cons_en.clone()
        }
        .into(),
        custom: provider.custom,
        active: addresses
            .iter()
            .filter(|value| !value.is_empty())
            .any(|value| current_dns.contains(value)),
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
    let rows = providers
        .iter()
        .map(|provider| row(provider, language, current_dns))
        .collect::<Vec<_>>();
    window.set_providers(ModelRc::from(Rc::new(VecModel::from(rows))));
}

fn show_message(window: &AppWindow, message: impl Into<SharedString>, error: bool) {
    window.set_toast_message(message.into());
    window.set_toast_error(error);
}

fn main() -> Result<(), slint::PlatformError> {
    let window = AppWindow::new()?;
    let mut settings = storage::load();
    if settings.language != "id" {
        settings.language = "en".into();
    }

    let mut initial = models::default_providers();
    initial.append(&mut settings.custom_providers);
    let providers = Rc::new(RefCell::new(initial));
    let language = Rc::new(RefCell::new(settings.language));
    let adapters = Arc::new(Mutex::new(Vec::<system::AdapterInfo>::new()));

    window.set_language(language.borrow().as_str().into());
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
                    window.set_busy(false);
                    match result {
                        Ok(found) => {
                            let labels = found
                                .iter()
                                .map(|item| SharedString::from(&item.name))
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
                            }
                            if let Ok(mut cached) = adapters.lock() {
                                *cached = found;
                            }
                        }
                        Err(message) => show_message(&window, message, true),
                    }
                });
            });
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
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
        window.on_clear_form(move || {
            if let Some(window) = weak.upgrade() {
                window.set_form_id("".into());
                window.set_form_name("".into());
                window.set_form_ipv4_primary("".into());
                window.set_form_ipv4_secondary("".into());
                window.set_form_ipv6_primary("".into());
                window.set_form_ipv6_secondary("".into());
                window.set_form_summary("".into());
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
            }
        });
    }

    {
        let providers = providers.clone();
        let language = language.clone();
        let weak = window.as_weak();
        window.on_save_provider(move |id, name, v4a, v4b, v6a, v6b, summary| {
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
                pros_en: "User-defined resolver".into(),
                pros_id: "Resolver ditentukan pengguna".into(),
                cons_en: "Trust and availability depend on its operator".into(),
                cons_id: "Kepercayaan dan ketersediaan bergantung pada pengelolanya".into(),
                custom: true,
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
        let weak = window.as_weak();
        window.on_delete_provider(move |id| {
            providers
                .borrow_mut()
                .retain(|provider| !provider.custom || provider.id != id.as_str());
            let list = providers.borrow();
            let settings = storage::Settings {
                language: language.borrow().clone(),
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

    window.invoke_refresh();
    window.run()
}
