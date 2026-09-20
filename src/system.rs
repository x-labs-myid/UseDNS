use std::{
    net::{IpAddr, SocketAddr, TcpStream},
    time::{Duration, Instant},
};

#[cfg(windows)]
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
use std::{os::windows::process::CommandExt, process::Stdio};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn hidden_output(command: &mut Command) -> std::io::Result<std::process::Output> {
    command
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
}

#[derive(Clone, Debug, Default)]
pub struct AdapterInfo {
    pub name: String,
    pub label: String,
    pub dns: String,
    pub interface_index: u32,
}

#[cfg(windows)]
pub fn prefers_dark_mode() -> bool {
    hidden_output(Command::new("reg.exe").args([
        "query",
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
        "/v",
        "AppsUseLightTheme",
    ]))
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
    let output = hidden_output(Command::new("powershell.exe").args([
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-Command",
        script,
    ]))
    .map_err(|error| format!("Could not start PowerShell: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if message.is_empty() {
            "The PowerShell operation failed.".into()
        } else {
            message
                .lines()
                .next()
                .unwrap_or("The PowerShell operation failed.")
                .into()
        })
    }
}

#[cfg(windows)]
fn elevated_powershell(script: &str) -> Result<(), String> {
    let guarded_script =
        format!("$ErrorActionPreference='Stop'; try {{ {script}; exit 0 }} catch {{ exit 1 }}");
    let encoded_bytes = guarded_script
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let encoded_script = BASE64.encode(encoded_bytes);
    let launcher = format!(
        "$ErrorActionPreference='Stop'; try {{ \
         $p=Start-Process -FilePath 'powershell.exe' -Verb RunAs -Wait -PassThru \
         -ArgumentList @('-NoProfile','-NonInteractive','-WindowStyle','Hidden','-EncodedCommand','{encoded_script}'); \
         if ($p.ExitCode -ne 0) {{ exit $p.ExitCode }} \
         }} catch {{ Write-Error $_.Exception.Message; exit 1 }}"
    );
    let output = hidden_output(Command::new("powershell.exe").args([
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-Command",
        &launcher,
    ]))
    .map_err(|error| format!("Could not request Administrator access: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let cancelled = stderr.contains("canceled by the user")
            || stderr.contains("cancelled by the user")
            || stderr.contains("dibatalkan oleh pengguna");
        Err(if cancelled {
            "Administrator permission was cancelled. DNS was not changed.".into()
        } else {
            "Could not change DNS. Approve the Administrator prompt and try again.".into()
        })
    }
}

#[cfg(windows)]
pub fn active_adapters() -> Result<Vec<AdapterInfo>, String> {
    let script = "$gw = (Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Sort-Object RouteMetric | Select-Object -ExpandProperty InterfaceAlias -First 1); Get-NetAdapter | Where-Object Status -eq 'Up' | Sort-Object { if ($_.Name -eq $gw) { 0 } elseif ($_.InterfaceDescription -match 'Virtual|Hyper-V|vEthernet|Loopback|TAP|VPN') { 2 } else { 1 } } | ForEach-Object { $n=$_.Name; $i=$_.ifIndex; $d=(Get-DnsClientServerAddress -InterfaceIndex $i).ServerAddresses -join ', '; $p=(Get-NetConnectionProfile -InterfaceIndex $i -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Name); $l=if ($p) { $n + ' — ' + $p } else { $n }; Write-Output ($n + [char]9 + $l + [char]9 + $d + [char]9 + $i) }";
    let output = powershell(script)?;
    let adapters = output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?;
            let label = fields.next()?;
            let dns = fields.next()?;
            let interface_index = fields.next()?.parse().ok()?;
            Some(AdapterInfo {
                name: name.into(),
                label: label.into(),
                dns: dns.into(),
                interface_index,
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
        label: "Default network".into(),
        dns: "System managed".into(),
        interface_index: 0,
    }])
}

#[cfg(windows)]
pub fn network_counters(interface_index: u32) -> Result<(u64, u64), String> {
    use windows::Win32::NetworkManagement::IpHelper::{GetIfEntry2, MIB_IF_ROW2};

    if interface_index == 0 {
        return Err("No network adapter was selected.".into());
    }

    // SAFETY: A zeroed MIB_IF_ROW2 with InterfaceIndex populated is the documented
    // input for GetIfEntry2. The API initializes the remaining fields in place.
    unsafe {
        let mut row: MIB_IF_ROW2 = std::mem::zeroed();
        row.InterfaceIndex = interface_index;
        GetIfEntry2(&mut row)
            .ok()
            .map_err(|error| format!("Could not read adapter statistics: {error}"))?;
        Ok((row.InOctets, row.OutOctets))
    }
}

#[cfg(not(windows))]
pub fn network_counters(_interface_index: u32) -> Result<(u64, u64), String> {
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
    elevated_powershell(&format!(
        "Set-DnsClientServerAddress -InterfaceAlias '{escaped_adapter}' -ServerAddresses ({quoted}) -ErrorAction Stop"
    ))
}

#[cfg(not(windows))]
pub fn apply_dns(_adapter: &str, _addresses: &[String]) -> Result<(), String> {
    Err("Changing DNS is currently supported on Windows only.".into())
}

#[cfg(windows)]
pub fn reset_dns(adapter: &str) -> Result<(), String> {
    let escaped_adapter = adapter.replace('\'', "''");
    elevated_powershell(&format!(
        "Set-DnsClientServerAddress -InterfaceAlias '{escaped_adapter}' -ResetServerAddresses -ErrorAction Stop"
    ))
}

#[cfg(not(windows))]
pub fn reset_dns(_adapter: &str) -> Result<(), String> {
    Err("Changing DNS is currently supported on Windows only.".into())
}

pub fn check_connection(target: &str) -> Result<u128, String> {
    let clean_target = target
        .split([',', ';', ' ', '\t'])
        .map(str::trim)
        .find(|part| !part.is_empty() && *part != "—")
        .unwrap_or("1.1.1.1");
    let ip: IpAddr = clean_target
        .parse()
        .map_err(|_| format!("Could not check the resolver address '{clean_target}'."))?;
    let started = Instant::now();
    for port in [53_u16, 443] {
        let addr = SocketAddr::new(ip, port);
        if TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok() {
            return Ok(started.elapsed().as_millis().max(1));
        }
    }
    Err("The DNS resolver did not respond.".into())
}
