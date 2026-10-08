use crate::AppWindow;
use slint::ComponentHandle;
use std::{
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Port {
    pub pid: u32,
    pub name: String,
    pub endpoint: String,
    pub protocol: String,
    #[serde(default)]
    pub started: String,
}

pub(crate) fn command(program: &str, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = bounded_output(
        &mut command,
        &AtomicBool::new(false),
        Duration::from_secs(120),
    )?;
    if program == "lsof" && output.status.code() == Some(1) && output.stderr.is_empty() {
        return Ok(String::new());
    }
    if !output.status.success() {
        return Err(format!(
            "{program}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().into())
}

fn bounded_output(
    command: &mut Command,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    use std::io::Read;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let start = std::time::Instant::now();
    let status = loop {
        if cancel.load(Ordering::Relaxed) || start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            break Err(if cancel.load(Ordering::Relaxed) {
                "Cancelled"
            } else {
                "Operation timed out"
            }
            .to_owned());
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(e.to_string());
            }
        }
    };
    let stdout = out
        .join()
        .map_err(|_| "Output reader failed")?
        .map_err(|e| e.to_string())?;
    let stderr = err
        .join()
        .map_err(|_| "Error reader failed")?
        .map_err(|e| e.to_string())?;
    Ok(std::process::Output {
        status: status?,
        stdout,
        stderr,
    })
}

pub fn ports() -> Result<Vec<Port>, String> {
    #[cfg(windows)]
    {
        crate::windows_ports::ports()
    }
    #[cfg(not(windows))]
    {
        let output = command("lsof", &["-nP", "-iTCP", "-sTCP:LISTEN", "-iUDP", "-FpcPn"])?;
        let mut rows = parse_lsof(&output);
        let processes = command("ps", &["-ax", "-o", "pid=,lstart="])?;
        let starts = processes
            .lines()
            .filter_map(|line| {
                let (pid, started) = line.trim().split_once(char::is_whitespace)?;
                Some((pid.parse::<u32>().ok()?, started.trim().to_owned()))
            })
            .collect::<std::collections::HashMap<_, _>>();
        for row in &mut rows {
            row.started = starts.get(&row.pid).cloned().unwrap_or_default();
        }
        Ok(rows)
    }
}

#[cfg(any(not(windows), test))]
fn parse_lsof(output: &str) -> Vec<Port> {
    let mut rows = Vec::new();
    let mut pid = 0;
    let mut name = String::new();
    let mut protocol = String::new();
    for line in output.lines() {
        if let Some(value) = line.strip_prefix('p') {
            pid = value.parse().unwrap_or(0);
            name.clear();
            protocol.clear();
        } else if let Some(value) = line.strip_prefix('c') {
            name = value.into();
        } else if let Some(value) = line.strip_prefix('P') {
            protocol = value.into();
        } else if let Some(value) = line.strip_prefix('n')
            && pid != 0
            && !value.contains("->")
            && (protocol == "TCP" || protocol == "UDP")
        {
            rows.push(Port {
                pid,
                name: name.clone(),
                protocol: protocol.clone(),
                endpoint: value.into(),
                started: String::new(),
            });
        }
    }
    rows
}

fn terminate(port: &Port, force: bool) -> Result<(), String> {
    if port.pid <= 4 || port.pid == std::process::id() {
        return Err("This process is protected.".into());
    }
    if port.started.is_empty() {
        return Err("Could not verify process identity. Termination is unavailable.".into());
    }
    if !ports()?.contains(port) {
        return Err("The process or port changed. Refresh and select it again.".into());
    }
    #[cfg(windows)]
    {
        let pid = port.pid.to_string();
        let mut args = vec!["/PID", &pid];
        if force {
            args.push("/F");
        }
        command("taskkill.exe", &args).map(|_| ())
    }
    #[cfg(not(windows))]
    {
        command(
            "kill",
            &[if force { "-KILL" } else { "-TERM" }, &port.pid.to_string()],
        )
        .map(|_| ())
    }
}

pub fn bind(window: &AppWindow) {
    window.set_winnat_supported(cfg!(windows));
    let rows = Arc::new(std::sync::Mutex::new(Vec::<Port>::new()));
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let weak = window.as_weak();
        let cancel = cancel.clone();
        let stop = cancel.clone();
        window.on_cancel_speedtest(move || {
            stop.store(true, Ordering::Relaxed);
        });
        window.on_start_speedtest(move || {
            let Some(app) = weak.upgrade() else { return };
            if app.get_speedtest_running() {
                return;
            }
            cancel.store(false, Ordering::Relaxed);
            app.set_speedtest_phase(1);
            app.set_speedtest_running(true);
            app.set_speedtest_result("".into());
            app.set_speedtest_download("—".into());
            app.set_speedtest_upload("—".into());
            app.set_speedtest_latency("—".into());
            let weak = weak.clone();
            let cancel = cancel.clone();
            std::thread::spawn(move || {
                let progress_window = weak.clone();
                let result = crate::speedtest::run(&cancel, move |phase, measured| {
                    let _ = progress_window.upgrade_in_event_loop(move |app| {
                        match phase {
                            1 => app.set_speedtest_latency(measured.into()),
                            2 => app.set_speedtest_download(measured.into()),
                            3 => app.set_speedtest_upload(measured.into()),
                            _ => {}
                        }
                        app.set_speedtest_phase(phase);
                    });
                });
                let _ = weak.upgrade_in_event_loop(move |app| {
                    app.set_speedtest_running(false);
                    match result {
                        Ok(value) => {
                            app.set_speedtest_phase(4);
                            app.set_speedtest_download(value.download.into());
                            app.set_speedtest_upload(value.upload.into());
                            app.set_speedtest_latency(value.latency.into());
                            app.set_speedtest_result("".into());
                        }
                        Err(error) => {
                            app.set_speedtest_phase(0);
                            app.set_speedtest_result(format!("Speedtest: {error}").into())
                        }
                    }
                });
            });
        });
    }
    {
        let weak = window.as_weak();
        let rows = rows.clone();
        window.on_refresh_ports(move || {
            let Some(app) = weak.upgrade() else { return };
            if app.get_tools_busy() || app.get_busy() {
                return;
            }
            app.set_tools_busy(true);
            app.set_tools_status(
                if app.get_language() == "id" {
                    "Memuat daftar port..."
                } else {
                    "Loading ports..."
                }
                .into(),
            );
            let weak = weak.clone();
            let rows = rows.clone();
            std::thread::spawn(move || {
                let result = ports();
                let _ = weak.upgrade_in_event_loop(move |app| {
                    app.set_tools_busy(false);
                    match result {
                        Ok(found) => {
                            app.set_port_labels(
                                std::rc::Rc::new(slint::VecModel::from(
                                    found
                                        .iter()
                                        .map(|p| {
                                            format!(
                                                "{}  {}  PID {}  {}",
                                                p.protocol, p.endpoint, p.pid, p.name
                                            )
                                            .into()
                                        })
                                        .collect::<Vec<slint::SharedString>>(),
                                ))
                                .into(),
                            );
                            *rows.lock().unwrap() = found;
                            app.set_tools_status("".into());
                        }
                        Err(e) => app.set_tools_status(e.into()),
                    }
                });
            });
        });
    }
    {
        let weak = window.as_weak();
        let rows = rows.clone();
        window.on_stop_port(move |index, force| {
            let Some(app) = weak.upgrade() else { return };
            if app.get_tools_busy() || app.get_busy() {
                return;
            }
            let Some(port) = rows.lock().unwrap().get(index as usize).cloned() else {
                return;
            };
            app.set_tools_busy(true);
            let weak = weak.clone();
            std::thread::spawn(move || {
                let result = terminate(&port, force);
                let _ = weak.upgrade_in_event_loop(move |app| {
                    app.set_tools_busy(false);
                    app.set_tools_status(
                        result
                            .map(|_| "Termination requested. Refresh ports to verify.".into())
                            .unwrap_or_else(|e| e)
                            .into(),
                    );
                });
            });
        });
    }
    {
        let weak = window.as_weak();
        window.on_restart_winnat(move || {
            let Some(app) = weak.upgrade() else { return };
            if app.get_tools_busy() || app.get_busy() {
                return;
            }
            app.set_tools_busy(true);
            app.set_busy(true);
            let weak = weak.clone();
            std::thread::spawn(move || {
                let result = crate::system::restart_winnat();
                let _ = weak.upgrade_in_event_loop(move |app| {
                    app.set_tools_busy(false);
                    app.set_busy(false);
                    app.set_tools_status(
                        result
                            .map(|_| "WinNAT restarted.".into())
                            .unwrap_or_else(|e| e)
                            .into(),
                    );
                });
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn reports_our_actual_tcp_listener_and_udp_endpoint() {
        let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let rows = ports().unwrap();
        for (protocol, endpoint) in [
            ("TCP", tcp.local_addr().unwrap().to_string()),
            ("UDP", udp.local_addr().unwrap().to_string()),
        ] {
            assert!(
                rows.iter().any(|p| p.pid == std::process::id()
                    && p.protocol == protocol
                    && p.endpoint == endpoint),
                "Missing {protocol} {endpoint}"
            );
        }
    }
    #[test]
    fn cancelled_command_stops_without_waiting_for_completion() {
        let mut command = if cfg!(windows) {
            let mut cmd = Command::new("powershell.exe");
            cmd.args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                "Start-Sleep -Seconds 10",
            ]);
            cmd
        } else {
            let mut cmd = Command::new("sleep");
            cmd.arg("10");
            cmd
        };
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let started = std::time::Instant::now();
        assert_eq!(
            bounded_output(
                &mut command,
                &AtomicBool::new(true),
                Duration::from_secs(15)
            )
            .unwrap_err(),
            "Cancelled"
        );
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn parses_listening_sockets_and_skips_connections() {
        let rows = parse_lsof(
            "p123\ncnode\nPTCP\nn*:3000\nn127.0.0.1:5000->127.0.0.1:3000\np456\ncdns\nPUDP\nn[::1]:53\n",
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].pid, 123);
        assert_eq!(rows[1].protocol, "UDP");
    }
    #[test]
    fn protects_self_and_system_processes() {
        for pid in [0, 4, std::process::id()] {
            assert!(
                terminate(
                    &Port {
                        pid,
                        name: String::new(),
                        endpoint: String::new(),
                        protocol: String::new(),
                        started: String::new(),
                    },
                    false
                )
                .is_err()
            );
        }
    }
}
