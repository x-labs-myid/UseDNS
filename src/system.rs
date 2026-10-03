use std::{
    net::IpAddr,
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
    hidden_output_with_timeout(command, Duration::from_secs(45))
}

#[cfg(windows)]
fn hidden_output_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> std::io::Result<std::process::Output> {
    use std::io::{self, Read};
    let mut child = command
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing stdout pipe"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("missing stderr pipe"))?;
    // Drain both pipes concurrently so a full stderr pipe cannot deadlock PowerShell.
    let read_stdout = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let read_stderr = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(20))
            }
            result => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(result.err().unwrap_or_else(|| {
                    io::Error::new(io::ErrorKind::TimedOut, "PowerShell operation timed out")
                }));
            }
        }
    };
    let stdout = read_stdout
        .join()
        .map_err(|_| io::Error::other("stdout reader failed"))?;
    let stderr = read_stderr
        .join()
        .map_err(|_| io::Error::other("stderr reader failed"))?;
    Ok(std::process::Output {
        status: status?,
        stdout: stdout?,
        stderr: stderr?,
    })
}

#[cfg(windows)]
#[derive(serde::Serialize, serde::Deserialize)]
enum ElevatedDnsOperation {
    Apply {
        adapter: String,
        addresses: Vec<String>,
        doh_template: Option<String>,
    },
    Reset {
        adapter: String,
    },
}

#[derive(Clone, Debug, Default)]
pub struct AdapterInfo {
    pub name: String,
    pub label: String,
    pub dns: String,
    pub interface_index: u32,
    pub dns_automatic: bool,
    pub dns_encrypted: bool,
}

#[cfg(windows)]
pub fn prefers_dark_mode() -> bool {
    use windows::{
        Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
        core::w,
    };
    let mut light: u32 = 1;
    let mut length = std::mem::size_of::<u32>() as u32;
    // SAFETY: Windows writes at most length bytes into a valid DWORD.
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut light as *mut u32).cast()),
            Some(&mut length),
        )
        .is_ok()
            && light == 0
    }
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
fn run_elevated_dns_operation(operation: ElevatedDnsOperation) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::{
            Foundation::CloseHandle,
            System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject},
            UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
        },
        core::PCWSTR,
    };

    let payload = serde_json::to_vec(&operation)
        .map(|bytes| BASE64.encode(bytes))
        .map_err(|error| format!("Could not prepare the DNS operation: {error}"))?;
    let executable =
        std::env::current_exe().map_err(|error| format!("Could not locate UseDNS: {error}"))?;
    let executable_wide = executable
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result_path = std::env::temp_dir().join(format!(
        "usedns-helper-{}-{}.result",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result_path_payload = BASE64.encode(result_path.to_string_lossy().as_bytes());
    let parameters_wide = format!("--dns-helper {payload} {result_path_payload}")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    // SAFETY: All strings are NUL-terminated and remain alive until ShellExecuteExW
    // returns. SEE_MASK_NOCLOSEPROCESS requests a process handle that is waited on
    // and closed below.
    unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS;
        info.lpVerb = windows::core::w!("runas");
        info.lpFile = PCWSTR(executable_wide.as_ptr());
        info.lpParameters = PCWSTR(parameters_wide.as_ptr());
        info.nShow = 0;

        ShellExecuteExW(&mut info).map_err(|error| {
            if error.code().0 as u32 == 0x8007_04c7 {
                "Administrator permission was cancelled. DNS was not changed.".to_string()
            } else {
                "Could not request Administrator access: ".to_owned() + &error.to_string()
            }
        })?;
        if info.hProcess.is_invalid() {
            return Err("Could not start the elevated UseDNS helper.".into());
        }

        WaitForSingleObject(info.hProcess, INFINITE);
        let mut exit_code = 1;
        let exit_result = GetExitCodeProcess(info.hProcess, &mut exit_code);
        let _ = CloseHandle(info.hProcess);
        exit_result.map_err(|error| format!("Could not read the DNS operation result: {error}"))?;

        let helper_message = std::fs::read_to_string(&result_path).unwrap_or_default();
        let _ = std::fs::remove_file(&result_path);
        if exit_code == 0 {
            Ok(())
        } else if helper_message.trim().is_empty() {
            Err("The elevated DNS helper failed without an error message.".into())
        } else {
            Err(helper_message.trim().into())
        }
    }
}

#[cfg(windows)]
pub fn run_dns_helper_if_requested() -> Option<i32> {
    let mut args = std::env::args();
    let _executable = args.next();
    if args.next().as_deref() != Some("--dns-helper") {
        return None;
    }
    let operation_payload = args.next();
    let result_path = args
        .next()
        .and_then(|payload| BASE64.decode(payload).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok());
    let result = operation_payload
        .ok_or_else(|| "The DNS helper request is incomplete.".to_string())
        .and_then(|payload| {
            BASE64
                .decode(payload)
                .map_err(|_| "The DNS helper request is invalid.".to_string())
        })
        .and_then(|bytes| {
            serde_json::from_slice::<ElevatedDnsOperation>(&bytes)
                .map_err(|_| "The DNS helper operation is invalid.".to_string())
        })
        .and_then(|operation| {
            let script = match operation {
                ElevatedDnsOperation::Apply {
                    adapter,
                    addresses,
                    doh_template,
                } => {
                    let adapter = adapter.replace('\'', "''");
                    let quoted_addresses = addresses
                        .iter()
                        .map(|address| "'".to_owned() + &address.replace('\'', "''") + "'")
                        .collect::<Vec<_>>()
                        .join(",");
                    let (encryption_setup, interface_encryption) =
                        if let Some(template) = doh_template {
                            let template = template.replace('\'', "''");
                            let encryption_setup = addresses
                                .iter()
                                .map(|address| {
                                    let address = address.replace('\'', "''");
                                    "$existing = Get-DnsClientDohServerAddress -ServerAddress '__ADDRESS__' -ErrorAction SilentlyContinue; if ($null -ne $existing) { Set-DnsClientDohServerAddress -ServerAddress '__ADDRESS__' -DohTemplate '__TEMPLATE__' -AllowFallbackToUdp $False -AutoUpgrade $True -ErrorAction Stop } else { Add-DnsClientDohServerAddress -ServerAddress '__ADDRESS__' -DohTemplate '__TEMPLATE__' -AllowFallbackToUdp $False -AutoUpgrade $True -ErrorAction Stop }; $configured = Get-DnsClientDohServerAddress -ServerAddress '__ADDRESS__' -ErrorAction Stop; if (-not $configured.AutoUpgrade -or $configured.AllowFallbackToUdp) { throw 'Windows did not enable encrypted DNS for __ADDRESS__.' }"
                                        .replace("__ADDRESS__", &address)
                                        .replace("__TEMPLATE__", &template)
                                })
                                .collect::<Vec<_>>()
                                .join("; ");
                            let interface_encryption = "Remove-Item -LiteralPath ($dohRoot + '\\Doh') -Recurse -Force -ErrorAction SilentlyContinue; Remove-Item -LiteralPath ($dohRoot + '\\Doh6') -Recurse -Force -ErrorAction SilentlyContinue; ".to_string()
                                + &addresses
                                    .iter()
                                    .filter_map(|address| {
                                        let parsed = address.parse::<IpAddr>().ok()?;
                                        let family = if parsed.is_ipv4() { "Doh" } else { "Doh6" };
                                        let address = address.replace('\'', "''");
                                        Some(
                                            "$dohKey = $dohRoot + '\\__FAMILY__\\__ADDRESS__'; New-Item -Path $dohKey -Force -ErrorAction Stop | Out-Null; New-ItemProperty -Path $dohKey -Name 'DohFlags' -PropertyType QWord -Value 17 -Force -ErrorAction Stop | Out-Null; New-ItemProperty -Path $dohKey -Name 'DohTemplate' -PropertyType String -Value '__TEMPLATE__' -Force -ErrorAction Stop | Out-Null; $saved = Get-ItemProperty -LiteralPath $dohKey -ErrorAction Stop; if ([uint64]$saved.DohFlags -ne 17 -or $saved.DohTemplate -ne '__TEMPLATE__') { throw 'Windows did not save encrypted DNS for __ADDRESS__.' }"
                                                .replace("__FAMILY__", family)
                                                .replace("__ADDRESS__", &address)
                                                .replace("__TEMPLATE__", &template),
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join("; ");
                            (encryption_setup, interface_encryption)
                        } else {
                            let interface_encryption = "Remove-Item -LiteralPath ($dohRoot + '\\Doh') -Recurse -Force -ErrorAction SilentlyContinue; Remove-Item -LiteralPath ($dohRoot + '\\Doh6') -Recurse -Force -ErrorAction SilentlyContinue".to_string();
                            ("Write-Output '' | Out-Null".to_string(), interface_encryption)
                        };
                    encryption_setup
                        + "; Set-DnsClientServerAddress -InterfaceAlias '"
                        + &adapter
                        + "' -ServerAddresses ("
                        + &quoted_addresses
                        + ") -ErrorAction Stop; $guid = (Get-NetAdapter -Name '"
                        + &adapter
                        + "' -ErrorAction Stop).InterfaceGuid; if ([string]::IsNullOrWhiteSpace([string]$guid)) { throw 'Could not resolve the network interface identifier.' }; $dohRoot = 'HKLM:\\SYSTEM\\CurrentControlSet\\Services\\Dnscache\\InterfaceSpecificParameters\\' + $guid + '\\DohInterfaceSettings'; New-Item -Path $dohRoot -Force -ErrorAction Stop | Out-Null; "
                        + &interface_encryption
                        + "; Clear-DnsClientCache -ErrorAction SilentlyContinue"
                }
                ElevatedDnsOperation::Reset { adapter } => {
                    let adapter = adapter.replace('\'', "''");
                    "Set-DnsClientServerAddress -InterfaceAlias '__ADAPTER__' -ResetServerAddresses -ErrorAction Stop; $guid = (Get-NetAdapter -Name '__ADAPTER__' -ErrorAction Stop).InterfaceGuid; $dohRoot = 'HKLM:\\SYSTEM\\CurrentControlSet\\Services\\Dnscache\\InterfaceSpecificParameters\\' + $guid + '\\DohInterfaceSettings'; Remove-Item -LiteralPath $dohRoot -Recurse -Force -ErrorAction SilentlyContinue; Clear-DnsClientCache -ErrorAction SilentlyContinue"
                        .replace("__ADAPTER__", &adapter)
                }
            };
            powershell(&script).map(|_| ())
        });
    if let (Err(message), Some(result_path)) = (&result, result_path) {
        let _ = std::fs::write(result_path, message);
    }
    Some(if result.is_ok() { 0 } else { 1 })
}

#[cfg(not(windows))]
pub fn run_dns_helper_if_requested() -> Option<i32> {
    None
}

#[cfg(windows)]
pub fn active_adapters() -> Result<Vec<AdapterInfo>, String> {
    let script = r"$gw = (Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Sort-Object RouteMetric | Select-Object -ExpandProperty InterfaceIndex -First 1); $profiles=@{}; Get-NetConnectionProfile -ErrorAction SilentlyContinue | ForEach-Object { $profiles[[int]$_.InterfaceIndex]=$_.Name }; $dns=@{}; Get-DnsClientServerAddress -ErrorAction SilentlyContinue | Group-Object InterfaceIndex | ForEach-Object { $dns[[int]$_.Name]=(($_.Group.ServerAddresses | Where-Object { $_ }) -join ', ') }; Get-NetAdapter | Where-Object Status -eq 'Up' | Sort-Object { if ($_.ifIndex -eq $gw) { 0 } elseif ($_.InterfaceDescription -match 'Virtual|Hyper-V|vEthernet|Loopback|TAP|VPN') { 2 } else { 1 } } | ForEach-Object { $n=$_.Name; $i=[int]$_.ifIndex; $p=$profiles[$i]; $l=if ($p) { $n + ' — ' + $p } else { $n }; $g=$_.InterfaceGuid; $v4=(Get-ItemProperty -LiteralPath ('HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\' + $g) -Name NameServer -ErrorAction SilentlyContinue).NameServer; $v6=(Get-ItemProperty -LiteralPath ('HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters\Interfaces\' + $g) -Name NameServer -ErrorAction SilentlyContinue).NameServer; $auto=[string]::IsNullOrWhiteSpace([string]$v4) -and [string]::IsNullOrWhiteSpace([string]$v6); $dr='HKLM:\SYSTEM\CurrentControlSet\Services\Dnscache\InterfaceSpecificParameters\' + $g; $flags=Get-ChildItem -LiteralPath ($dr + '\DohInterfaceSettings') -Recurse -ErrorAction SilentlyContinue | ForEach-Object { (Get-ItemProperty -LiteralPath $_.PSPath -Name DohFlags -ErrorAction SilentlyContinue).DohFlags }; $enc=$flags -contains 17; Write-Output ($n + [char]9 + $l + [char]9 + $dns[$i] + [char]9 + $i + [char]9 + $(if ($auto) { '1' } else { '0' }) + [char]9 + $(if ($enc) { '1' } else { '0' })) }";
    let output = powershell(script)?;
    let adapters = output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?;
            let label = fields.next()?;
            let dns = fields.next()?;
            let interface_index = fields.next()?.parse().ok()?;
            let dns_automatic = fields.next() == Some("1");
            let dns_encrypted = fields.next() == Some("1");
            Some(AdapterInfo {
                name: name.into(),
                label: label.into(),
                dns: dns.into(),
                interface_index,
                dns_automatic,
                dns_encrypted,
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
        dns_automatic: true,
        dns_encrypted: false,
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
pub fn apply_dns(
    adapter: &str,
    addresses: &[String],
    doh_template: Option<&str>,
) -> Result<(), String> {
    if addresses.is_empty() {
        return Err("No DNS address was selected.".into());
    }
    if adapter.trim().is_empty()
        || addresses
            .iter()
            .any(|address| address.parse::<IpAddr>().is_err())
    {
        return Err("The network adapter or DNS addresses are invalid.".into());
    }
    if doh_template.is_some_and(|template| !template.starts_with("https://")) {
        return Err("Encrypted DNS requires an HTTPS template.".into());
    }
    run_elevated_dns_operation(ElevatedDnsOperation::Apply {
        adapter: adapter.into(),
        addresses: addresses.to_vec(),
        doh_template: doh_template.map(str::to_owned),
    })
}

#[cfg(not(windows))]
pub fn apply_dns(
    _adapter: &str,
    _addresses: &[String],
    _doh_template: Option<&str>,
) -> Result<(), String> {
    Err("Changing DNS is currently supported on Windows only.".into())
}

#[cfg(windows)]
pub fn reset_dns(adapter: &str) -> Result<(), String> {
    run_elevated_dns_operation(ElevatedDnsOperation::Reset {
        adapter: adapter.into(),
    })
}

#[cfg(not(windows))]
pub fn reset_dns(_adapter: &str) -> Result<(), String> {
    Err("Changing DNS is currently supported on Windows only.".into())
}

pub fn check_connection(target: &str) -> Result<u128, String> {
    check_configured_resolvers(target, crate::booster::resolver_latency)
}

fn check_configured_resolvers(
    target: &str,
    mut probe: impl FnMut(IpAddr) -> Result<u128, String>,
) -> Result<u128, String> {
    let mut addresses = Vec::new();
    for ip in target
        .split([',', ';', ' ', '\t'])
        .filter_map(|part| part.trim().parse::<IpAddr>().ok())
    {
        if !addresses.contains(&ip) {
            addresses.push(ip);
        }
    }
    if addresses.is_empty() {
        return Err("No configured DNS resolver is available to test.".into());
    }
    for ip in addresses.into_iter().take(4) {
        if let Ok(ms) = probe(ip) {
            return Ok(ms);
        }
    }
    Err(
        "Could not verify the configured DNS resolvers. UDP DNS may be unavailable or blocked."
            .into(),
    )
}

#[cfg(test)]
mod resolver_tests {
    use super::*;

    #[test]
    fn tries_secondary_dns_when_router_probe_fails() {
        let mut tested = Vec::new();
        let result = check_configured_resolvers("192.168.1.1, 192.168.1.1; ::1", |ip| {
            tested.push(ip);
            if ip.is_ipv6() {
                Ok(12)
            } else {
                Err("no response".into())
            }
        });
        assert_eq!(result.unwrap(), 12);
        assert_eq!(tested.len(), 2);
    }

    #[test]
    fn missing_dns_never_tests_an_unconfigured_public_resolver() {
        let result =
            check_configured_resolvers("—", |_| panic!("must not probe unconfigured DNS"));
        assert!(result.is_err());
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn terminates_a_stalled_powershell_process() {
        let error = hidden_output_with_timeout(
            Command::new("powershell.exe").args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                "Start-Sleep -Seconds 10",
            ]),
            Duration::from_millis(100),
        )
        .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    }
}
