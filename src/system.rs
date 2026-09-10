use std::{process::Command, time::Instant};

#[derive(Clone, Debug, Default)]
pub struct AdapterInfo {
    pub name: String,
    pub dns: String,
}

#[cfg(windows)]
pub fn prefers_dark_mode() -> bool {
    Command::new("reg.exe")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "/v",
            "AppsUseLightTheme",
        ])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("0x0"))
        .unwrap_or(false)
}

#[cfg(not(windows))]
pub fn prefers_dark_mode() -> bool {
    false
}

#[cfg(windows)]
fn powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|error| format!("Could not start PowerShell: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if message.is_empty() {
            "The DNS operation failed. Try running UseDNS as Administrator.".into()
        } else {
            message
        })
    }
}

#[cfg(windows)]
pub fn active_adapters() -> Result<Vec<AdapterInfo>, String> {
    let script = "$gw = (Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Sort-Object RouteMetric | Select-Object -ExpandProperty InterfaceAlias -First 1); Get-NetAdapter | Where-Object Status -eq 'Up' | Sort-Object { if ($_.Name -eq $gw) { 0 } elseif ($_.InterfaceDescription -match 'Virtual|Hyper-V|vEthernet|Loopback|TAP|VPN') { 2 } else { 1 } } | ForEach-Object { $n=$_.Name; $d=(Get-DnsClientServerAddress -InterfaceIndex $_.ifIndex).ServerAddresses -join ', '; Write-Output ($n + [char]9 + $d) }";
    let output = powershell(script)?;
    let adapters = output
        .lines()
        .filter_map(|line| {
            let (name, dns) = line.split_once('\t')?;
            Some(AdapterInfo {
                name: name.into(),
                dns: dns.into(),
            })
        })
        .collect::<Vec<_>>();
    if adapters.is_empty() {
        Err("No active network adapter was found.".into())
    } else {
        Ok(adapters)
    }
}

#[cfg(not(windows))]
pub fn active_adapters() -> Result<Vec<AdapterInfo>, String> {
    Ok(vec![AdapterInfo {
        name: "Default network".into(),
        dns: "System managed".into(),
    }])
}

#[cfg(windows)]
pub fn network_counters(adapter: &str) -> Result<(u64, u64), String> {
    let escaped_adapter = adapter.replace('\'', "''");
    let output = powershell(&format!(
        "$s=Get-NetAdapterStatistics -Name '{escaped_adapter}'; Write-Output ($s.ReceivedBytes.ToString() + [char]9 + $s.SentBytes.ToString())"
    ))?;
    let (received, sent) = output
        .split_once('\t')
        .ok_or_else(|| "Windows returned invalid network statistics.".to_string())?;
    Ok((
        received
            .trim()
            .parse()
            .map_err(|_| "Invalid received-byte counter.".to_string())?,
        sent.trim()
            .parse()
            .map_err(|_| "Invalid sent-byte counter.".to_string())?,
    ))
}

#[cfg(not(windows))]
pub fn network_counters(_adapter: &str) -> Result<(u64, u64), String> {
    Err("Real-time network activity is currently supported on Windows only.".into())
}

#[cfg(windows)]
pub fn apply_dns(adapter: &str, addresses: &[String]) -> Result<(), String> {
    if addresses.is_empty() {
        return Err("No DNS address was selected.".into());
    }
    let escaped_adapter = adapter.replace('\'', "''");
    let quoted = addresses
        .iter()
        .map(|address| format!("'{}'", address.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    powershell(&format!(
        "Set-DnsClientServerAddress -InterfaceAlias '{escaped_adapter}' -ServerAddresses ({quoted})"
    ))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn apply_dns(_adapter: &str, _addresses: &[String]) -> Result<(), String> {
    Err("Changing DNS is currently supported on Windows only.".into())
}

#[cfg(windows)]
pub fn reset_dns(adapter: &str) -> Result<(), String> {
    let escaped_adapter = adapter.replace('\'', "''");
    powershell(&format!(
        "Set-DnsClientServerAddress -InterfaceAlias '{escaped_adapter}' -ResetServerAddresses"
    ))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn reset_dns(_adapter: &str) -> Result<(), String> {
    Err("Changing DNS is currently supported on Windows only.".into())
}

pub fn check_connection(target: &str) -> Result<u128, String> {
    let clean_target = target.split(',').next().unwrap_or("1.1.1.1").trim();
    let target = if clean_target.is_empty() {
        "1.1.1.1"
    } else {
        clean_target
    };
    let started = Instant::now();
    #[cfg(windows)]
    let status = Command::new("ping")
        .args(["-n", "2", "-w", "2000", target])
        .status();
    #[cfg(not(windows))]
    let status = Command::new("ping")
        .args(["-c", "2", "-W", "2", target])
        .status();
    match status {
        Ok(result) if result.success() => Ok(started.elapsed().as_millis() / 2),
        Ok(_) => Err("The DNS resolver did not respond.".into()),
        Err(error) => Err(format!("Could not run the connection check: {error}")),
    }
}
