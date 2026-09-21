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

#[cfg(windows)]
#[derive(serde::Serialize, serde::Deserialize)]
enum ElevatedDnsOperation {
    Apply {
        adapter: String,
        addresses: Vec<String>,
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
    let parameters_wide = format!("--dns-helper {payload}")
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
                format!("Could not request Administrator access: {error}")
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

        if exit_code == 0 {
            Ok(())
        } else {
            Err("Could not change DNS with Administrator permission.".into())
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
    let result = args
        .next()
        .ok_or(())
        .and_then(|payload| BASE64.decode(payload).map_err(|_| ()))
        .and_then(|bytes| serde_json::from_slice::<ElevatedDnsOperation>(&bytes).map_err(|_| ()))
        .and_then(|operation| {
            let script = match operation {
                ElevatedDnsOperation::Apply { adapter, addresses } => {
                    let adapter = adapter.replace('\'', "''");
                    let addresses = addresses
                        .iter()
                        .map(|address| format!("'{}'", address.replace('\'', "''")))
                        .collect::<Vec<_>>()
                        .join(",");
                    format!("Set-DnsClientServerAddress -InterfaceAlias '{adapter}' -ServerAddresses ({addresses}) -ErrorAction Stop")
                }
                ElevatedDnsOperation::Reset { adapter } => {
                    let adapter = adapter.replace('\'', "''");
                    format!("Set-DnsClientServerAddress -InterfaceAlias '{adapter}' -ResetServerAddresses -ErrorAction Stop")
                }
            };
            powershell(&script).map(|_| ()).map_err(|_| ())
        });
    Some(if result.is_ok() { 0 } else { 1 })
}

#[cfg(not(windows))]
pub fn run_dns_helper_if_requested() -> Option<i32> {
    None
}

#[cfg(windows)]
pub fn active_adapters() -> Result<Vec<AdapterInfo>, String> {
    let script = "$gw = (Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Sort-Object RouteMetric | Select-Object -ExpandProperty InterfaceIndex -First 1); $profiles=@{}; Get-NetConnectionProfile -ErrorAction SilentlyContinue | ForEach-Object { $profiles[[int]$_.InterfaceIndex]=$_.Name }; $dns=@{}; Get-DnsClientServerAddress -ErrorAction SilentlyContinue | Group-Object InterfaceIndex | ForEach-Object { $dns[[int]$_.Name]=(($_.Group.ServerAddresses | Where-Object { $_ }) -join ', ') }; Get-NetAdapter | Where-Object Status -eq 'Up' | Sort-Object { if ($_.ifIndex -eq $gw) { 0 } elseif ($_.InterfaceDescription -match 'Virtual|Hyper-V|vEthernet|Loopback|TAP|VPN') { 2 } else { 1 } } | ForEach-Object { $n=$_.Name; $i=[int]$_.ifIndex; $p=$profiles[$i]; $l=if ($p) { $n + ' — ' + $p } else { $n }; Write-Output ($n + [char]9 + $l + [char]9 + $dns[$i] + [char]9 + $i) }";
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
    run_elevated_dns_operation(ElevatedDnsOperation::Apply {
        adapter: adapter.into(),
        addresses: addresses.to_vec(),
    })
}

#[cfg(not(windows))]
pub fn apply_dns(_adapter: &str, _addresses: &[String]) -> Result<(), String> {
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
