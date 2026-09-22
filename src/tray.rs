use crate::models::DnsProvider;
use std::collections::HashMap;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
};

pub const OPEN_ID: &str = "tray-open";
pub const CLOSE_ID: &str = "tray-close";

pub struct TrayState {
    pub icon: TrayIcon,
    pub dns_actions: HashMap<String, (String, usize)>,
    pub dns_items: HashMap<String, CheckMenuItem>,
    pub provider_menus: HashMap<String, (String, Submenu)>,
}

pub fn create(providers: &[DnsProvider], language: &str) -> Result<TrayState, String> {
    let image = image::load_from_memory(include_bytes!("../.assets/UseDNS.png"))
        .map_err(|error| "Could not decode the tray icon: ".to_owned() + &error.to_string())?
        .into_rgba8();
    let (width, height) = image.dimensions();
    let icon = Icon::from_rgba(image.into_raw(), width, height)
        .map_err(|error| "Could not prepare the tray icon: ".to_owned() + &error.to_string())?;

    let menu = Menu::new();
    let open = MenuItem::with_id(
        OPEN_ID,
        if language == "id" { "Buka" } else { "Open" },
        true,
        None,
    );
    let change_dns = Submenu::new(
        if language == "id" {
            "Ganti DNS"
        } else {
            "Change DNS"
        },
        true,
    );
    let close = MenuItem::with_id(
        CLOSE_ID,
        if language == "id" { "Tutup" } else { "Close" },
        true,
        None,
    );
    menu.append(&open)
        .map_err(|error| "Could not build the tray menu: ".to_owned() + &error.to_string())?;

    let mut dns_actions = HashMap::new();
    let mut dns_items = HashMap::new();
    let mut provider_menus = HashMap::new();
    for provider in providers {
        let profiles = provider.get_profiles();
        if profiles.len() > 1 {
            let provider_menu = Submenu::new(&provider.name, true);
            for (index, profile) in profiles.iter().enumerate() {
                let id = "tray-dns:".to_owned() + &provider.id + ":" + &index.to_string();
                let label = if language == "id" {
                    &profile.name_id
                } else {
                    &profile.name_en
                };
                let item = CheckMenuItem::with_id(&id, label, true, false, None);
                provider_menu.append(&item).map_err(|error| {
                    "Could not build the tray DNS menu: ".to_owned() + &error.to_string()
                })?;
                dns_actions.insert(id.clone(), (provider.id.clone(), index));
                dns_items.insert(id, item);
            }
            change_dns.append(&provider_menu).map_err(|error| {
                "Could not build the tray DNS menu: ".to_owned() + &error.to_string()
            })?;
            provider_menus.insert(provider.id.clone(), (provider.name.clone(), provider_menu));
        } else {
            let id = "tray-dns:".to_owned() + &provider.id + ":0";
            let item = CheckMenuItem::with_id(&id, &provider.name, true, false, None);
            change_dns.append(&item).map_err(|error| {
                "Could not build the tray DNS menu: ".to_owned() + &error.to_string()
            })?;
            dns_actions.insert(id.clone(), (provider.id.clone(), 0));
            dns_items.insert(id, item);
        }
    }

    menu.append(&change_dns)
        .and_then(|_| menu.append(&close))
        .map_err(|error| "Could not build the tray menu: ".to_owned() + &error.to_string())?;

    let icon = TrayIconBuilder::new()
        .with_tooltip("UseDNS")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .map_err(|error| "Could not create the tray icon: ".to_owned() + &error.to_string())?;

    Ok(TrayState {
        icon,
        dns_actions,
        dns_items,
        provider_menus,
    })
}
