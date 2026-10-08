//! Platform DNS backends. Never overwrite resolv.conf or restart networking.
use crate::system::AdapterInfo;

fn run(program: &str, args: &[&str]) -> Result<String, String> {
    if program == "nmcli" && !args.contains(&"--escape") {
        let mut plain = vec!["--escape", "no"];
        plain.extend_from_slice(args);
        crate::network_tools::command(program, &plain)
    } else {
        crate::network_tools::command(program, args)
    }
}

pub fn adapters() -> Result<Vec<AdapterInfo>, String> {
    let mut adapters = Vec::new();
    if cfg!(target_os = "linux") {
        let names = run(
            "nmcli",
            &["-t", "--escape", "no", "-f", "DEVICE", "device", "status"],
        )?;
        for name in names.lines().filter(|n| *n != "lo") {
            let connection = run("nmcli", &["-g", "GENERAL.CON-UUID", "device", "show", name])?;
            if connection.is_empty() || connection == "--" {
                continue;
            }
            let dns = run("nmcli", &["-g", "IP4.DNS,IP6.DNS", "device", "show", name])?;
            let automatic = run(
                "nmcli",
                &[
                    "-g",
                    "ipv4.ignore-auto-dns,ipv6.ignore-auto-dns",
                    "connection",
                    "show",
                    "uuid",
                    &connection,
                ],
            )?;
            let manual = run(
                "nmcli",
                &[
                    "-g",
                    "ipv4.dns,ipv6.dns",
                    "connection",
                    "show",
                    "uuid",
                    &connection,
                ],
            )?;
            adapters.push(AdapterInfo {
                name: name.into(),
                label: name.into(),
                dns: dns.lines().collect::<Vec<_>>().join(", "),
                dns_automatic: manual.trim().is_empty() && automatic.lines().all(|v| v == "no"),
                ..Default::default()
            });
        }
    } else if cfg!(target_os = "macos") {
        let names = run("/usr/sbin/networksetup", &["-listallnetworkservices"])?;
        let order = run("/usr/sbin/networksetup", &["-listnetworkserviceorder"])?;
        let default_device =
            run("/sbin/route", &["-n", "get", "default"])
                .ok()
                .and_then(|output| {
                    output
                        .lines()
                        .find_map(|line| line.trim().strip_prefix("interface: ").map(str::to_owned))
                });
        let mut primary_service = String::new();
        for name in names.lines().skip(1).filter(|n| !n.starts_with('*')) {
            let configured = run("/usr/sbin/networksetup", &["-getdnsservers", name])?;
            let automatic = configured.contains("aren't any DNS Servers");
            // The effective DHCP resolver belongs to the device, not the service.
            let info = run("/usr/sbin/networksetup", &["-getinfo", name])?;
            if !info.lines().any(|line| {
                (line.starts_with("IP address:") || line.starts_with("IPv6 IP address:"))
                    && !line.ends_with("none")
                    && !line.ends_with("0.0.0.0")
            }) {
                continue;
            }
            let Some(device) = service_device(&order, name) else {
                continue;
            };
            let link = run("/sbin/ifconfig", &[&device])?;
            if link.lines().any(|line| line.trim() == "status: inactive") {
                continue;
            }
            if default_device.as_deref() == Some(device.as_str()) {
                primary_service = name.to_owned();
            }
            let dns = if automatic {
                let scoped = run("/usr/sbin/scutil", &["--dns"])
                    .map(|output| scoped_dns(&output, &device))
                    .unwrap_or_default();
                if !scoped.is_empty() {
                    scoped
                } else {
                    run(
                        "/usr/sbin/ipconfig",
                        &["getoption", &device, "domain_name_server"],
                    )
                    .unwrap_or_default()
                }
            } else {
                configured.lines().collect::<Vec<_>>().join(", ")
            };
            adapters.push(AdapterInfo {
                name: name.into(),
                label: name.into(),
                dns,
                dns_automatic: automatic,
                ..Default::default()
            });
        }
        adapters.sort_by_key(|adapter| adapter.name != primary_service);
    } else {
        return Err("DNS configuration is unsupported on this operating system.".into());
    }
    if adapters.is_empty() {
        Err("No supported active network adapter was found.".into())
    } else {
        Ok(adapters)
    }
}

pub fn apply(adapter: &str, addresses: &[String], doh: Option<&str>) -> Result<(), String> {
    if adapter.is_empty()
        || addresses.is_empty()
        || addresses
            .iter()
            .any(|a| a.parse::<std::net::IpAddr>().is_err())
    {
        return Err("Invalid adapter or DNS addresses.".into());
    }
    if doh.is_some() {
        return Err("Encrypted DNS is not supported by this platform backend. Disable encrypted DNS to apply plain DNS.".into());
    }
    change(adapter, Some(addresses))
}

pub fn reset(adapter: &str) -> Result<(), String> {
    change(adapter, None)
}

fn change(adapter: &str, addresses: Option<&[String]>) -> Result<(), String> {
    if !adapters()?.iter().any(|a| a.name == adapter) {
        return Err("The selected adapter is no longer active.".into());
    }
    if cfg!(target_os = "linux") {
        let uuid = run(
            "nmcli",
            &["-g", "GENERAL.CON-UUID", "device", "show", adapter],
        )?;
        let fields = [
            "ipv4.dns",
            "ipv4.ignore-auto-dns",
            "ipv6.dns",
            "ipv6.ignore-auto-dns",
        ];
        let old = fields
            .iter()
            .map(|f| run("nmcli", &["-g", f, "connection", "show", "uuid", &uuid]))
            .collect::<Result<Vec<_>, _>>()?;
        let old = old
            .into_iter()
            .enumerate()
            .map(|(i, value)| {
                if i % 2 == 0 {
                    value.lines().collect::<Vec<_>>().join(",")
                } else {
                    value
                }
            })
            .collect::<Vec<_>>();
        let mut values = Vec::new();
        for ipv4 in [true, false] {
            let ips = addresses
                .unwrap_or(&[])
                .iter()
                .filter(|a| {
                    a.parse::<std::net::IpAddr>()
                        .is_ok_and(|ip| ip.is_ipv4() == ipv4)
                })
                .cloned()
                .collect::<Vec<_>>();
            values.push(ips.join(","));
            values.push(if addresses.is_some() { "yes" } else { "no" }.into());
        }
        let modify = |values: &[String]| {
            let mut args = vec!["nmcli", "connection", "modify", "uuid", &uuid];
            for (field, value) in fields.iter().zip(values) {
                args.extend([*field, value.as_str()]);
            }
            run("pkexec", &args)
        };
        modify(&values)?;
        if let Err(error) = run("pkexec", &["nmcli", "device", "reapply", adapter]) {
            let rollback =
                modify(&old).and_then(|_| run("pkexec", &["nmcli", "device", "reapply", adapter]));
            return Err(format!(
                "Could not activate DNS: {error}. Rollback: {}",
                rollback.err().unwrap_or_else(|| "completed".into())
            ));
        }
        Ok(())
    } else {
        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        let values = addresses
            .map(|a| a.iter().map(|v| quote(v)).collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| "Empty".into());
        let shell = format!(
            "/usr/sbin/networksetup -setdnsservers {} {values}",
            quote(adapter)
        );
        let script = format!(
            "do shell script \"{}\" with administrator privileges",
            shell.replace('\\', "\\\\").replace('"', "\\\"")
        );
        run("/usr/bin/osascript", &["-e", &script]).map(|_| ())
    }
}

fn service_device(order: &str, service: &str) -> Option<String> {
    let mut selected = false;
    for line in order.lines() {
        if line.starts_with('(') && line.contains(") ") {
            selected = line
                .split_once(") ")
                .is_some_and(|(_, name)| name == service);
        } else if selected && let Some((_, device)) = line.split_once("Device: ") {
            return Some(device.trim_end_matches(')').to_owned());
        }
    }
    None
}

fn scoped_dns(output: &str, device: &str) -> String {
    let mut result = Vec::<String>::new();
    let mut addresses = Vec::new();
    let mut selected = false;
    for line in output.lines().chain(std::iter::once("resolver #end")) {
        let line = line.trim();
        if line.starts_with("resolver #") {
            if selected {
                result.append(&mut addresses);
            }
            addresses.clear();
            selected = false;
        } else if line.starts_with("if_index") {
            selected = line.ends_with(&format!("({device})"));
        } else if line.starts_with("nameserver[")
            && let Some((_, ip)) = line.split_once(" : ")
            && ip.parse::<std::net::IpAddr>().is_ok()
        {
            addresses.push(ip.to_owned());
        }
    }
    result.sort();
    result.dedup();
    result.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn macos_service_mapping_keeps_similar_names_separate() {
        let order = "(1) Wi-Fi 2\n(Hardware Port: Wi-Fi, Device: en1)\n(2) Wi-Fi\n(Hardware Port: Wi-Fi, Device: en0)\n";
        assert_eq!(service_device(order, "Wi-Fi").as_deref(), Some("en0"));
        assert!(service_device(order, "Ethernet").is_none());
    }
    #[test]
    fn macos_dns_is_scoped_to_selected_interface() {
        let output = "resolver #1\n nameserver[0] : 9.9.9.9\n if_index : 4 (en1)\nresolver #2\n nameserver[0] : 1.1.1.1\n nameserver[1] : 2606:4700:4700::1111\n if_index : 6 (en0)\n";
        assert_eq!(scoped_dns(output, "en0"), "1.1.1.1, 2606:4700:4700::1111");
        assert!(scoped_dns(output, "en2").is_empty());
    }
    #[test]
    fn rejects_invalid_requests_before_platform_commands() {
        assert!(apply("", &["1.1.1.1".into()], None).is_err());
        assert!(apply("eth0", &["invalid".into()], None).is_err());
        assert!(
            apply(
                "eth0",
                &["1.1.1.1".into()],
                Some("https://example.com/dns-query")
            )
            .is_err()
        );
        let _ = adapters as fn() -> Result<Vec<AdapterInfo>, String>;
        let _ = reset as fn(&str) -> Result<(), String>;
    }
}
