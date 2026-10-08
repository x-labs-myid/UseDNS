//! Read TCP/UDP ownership directly, without CIM, elevation, or PowerShell.
use crate::network_tools::Port;
use std::{
    collections::HashMap,
    mem::{offset_of, size_of},
    net::{Ipv4Addr, Ipv6Addr},
};
use windows::Win32::{
    Foundation::{CloseHandle, FILETIME},
    NetworkManagement::IpHelper::*,
    System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    },
};

fn process_identity(pid: u32) -> (String, String) {
    // SAFETY: Query-only process handle; buffers and FILETIMEs remain valid for
    // each call. Every successful OpenProcess is paired with CloseHandle.
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return (format!("PID {pid}"), String::new());
        };
        let mut image = vec![0u16; 32768];
        let mut length = image.len() as u32;
        let name = if QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(image.as_mut_ptr()),
            &mut length,
        )
        .is_ok()
        {
            let path = String::from_utf16_lossy(&image[..length as usize]);
            std::path::Path::new(&path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("PID {pid}"))
        } else {
            format!("PID {pid}")
        };
        let mut created = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        let started =
            if GetProcessTimes(handle, &mut created, &mut exit, &mut kernel, &mut user).is_ok() {
                (((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64).to_string()
            } else {
                String::new()
            };
        let _ = CloseHandle(handle);
        (name, started)
    }
}

fn table<R: Copy>(
    offset: usize,
    mut get: impl FnMut(Option<*mut core::ffi::c_void>, &mut u32) -> u32,
) -> Result<Vec<R>, String> {
    let mut size = 0;
    let status = get(None, &mut size);
    if status != 0 && status != 122 {
        return Err(format!(
            "Could not read socket table (Windows error {status})."
        ));
    }
    // A socket may be added between sizing and reading; retry boundedly.
    for _ in 0..4 {
        let mut buffer = vec![0u64; (size as usize).div_ceil(8).max(1)];
        let capacity = buffer.len() * size_of::<u64>();
        size = capacity as u32;
        let status = get(Some(buffer.as_mut_ptr().cast()), &mut size);
        if status == 122 {
            continue;
        }
        if status != 0 {
            return Err(format!(
                "Could not read socket table (Windows error {status})."
            ));
        }
        if size as usize > capacity || (size as usize) < size_of::<u32>() {
            return Err("Invalid socket table size.".into());
        }
        // SAFETY: The native API initialized the first size bytes. Count and
        // row bounds are checked before any row is copied. Reading unaligned
        // handles native table layout without creating misaligned references.
        unsafe {
            let bytes = buffer.as_ptr().cast::<u8>();
            let count = bytes.cast::<u32>().read_unaligned() as usize;
            let end = count
                .checked_mul(size_of::<R>())
                .and_then(|length| offset.checked_add(length));
            if end.is_none_or(|end| end > size as usize) {
                return Err("Invalid socket table bounds.".into());
            }
            return Ok((0..count)
                .map(|index| {
                    bytes
                        .add(offset + index * size_of::<R>())
                        .cast::<R>()
                        .read_unaligned()
                })
                .collect());
        }
    }
    Err("Socket table changed repeatedly. Refresh ports to retry.".into())
}

pub fn ports() -> Result<Vec<Port>, String> {
    let mut result = Vec::new();
    let mut processes = HashMap::new();
    let mut add = |pid, endpoint, protocol: &str| {
        let (name, started) = processes
            .entry(pid)
            .or_insert_with(|| process_identity(pid));
        result.push(Port {
            pid,
            endpoint,
            protocol: protocol.into(),
            name: name.clone(),
            started: started.clone(),
        });
    };
    // SAFETY: table supplies an aligned writable buffer and valid size pointer;
    // the requested owner-PID table classes match the row types and offsets.
    let tcp4 = table::<MIB_TCPROW_OWNER_PID>(
        offset_of!(MIB_TCPTABLE_OWNER_PID, table),
        |buffer, size| unsafe {
            GetExtendedTcpTable(buffer, size, true, 2, TCP_TABLE_OWNER_PID_LISTENER, 0)
        },
    )?;
    for row in tcp4 {
        add(
            row.dwOwningPid,
            format!(
                "{}:{}",
                Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes()),
                u16::from_be(row.dwLocalPort as u16)
            ),
            "TCP",
        );
    }
    let tcp6 = table::<MIB_TCP6ROW_OWNER_PID>(
        offset_of!(MIB_TCP6TABLE_OWNER_PID, table),
        |buffer, size| unsafe {
            GetExtendedTcpTable(buffer, size, true, 23, TCP_TABLE_OWNER_PID_LISTENER, 0)
        },
    )?;
    for row in tcp6 {
        add(
            row.dwOwningPid,
            ipv6_endpoint(row.ucLocalAddr, row.dwLocalScopeId, row.dwLocalPort),
            "TCP",
        );
    }
    let udp4 = table::<MIB_UDPROW_OWNER_PID>(
        offset_of!(MIB_UDPTABLE_OWNER_PID, table),
        |buffer, size| unsafe {
            GetExtendedUdpTable(buffer, size, true, 2, UDP_TABLE_OWNER_PID, 0)
        },
    )?;
    for row in udp4 {
        add(
            row.dwOwningPid,
            format!(
                "{}:{}",
                Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes()),
                u16::from_be(row.dwLocalPort as u16)
            ),
            "UDP",
        );
    }
    let udp6 = table::<MIB_UDP6ROW_OWNER_PID>(
        offset_of!(MIB_UDP6TABLE_OWNER_PID, table),
        |buffer, size| unsafe {
            GetExtendedUdpTable(buffer, size, true, 23, UDP_TABLE_OWNER_PID, 0)
        },
    )?;
    for row in udp6 {
        add(
            row.dwOwningPid,
            ipv6_endpoint(row.ucLocalAddr, row.dwLocalScopeId, row.dwLocalPort),
            "UDP",
        );
    }
    result
        .sort_by(|a, b| (&a.protocol, &a.endpoint, a.pid).cmp(&(&b.protocol, &b.endpoint, b.pid)));
    result.dedup();
    Ok(result)
}

fn ipv6_endpoint(address: [u8; 16], scope: u32, port: u32) -> String {
    let address = Ipv6Addr::from(address);
    let port = u16::from_be(port as u16);
    if scope == 0 {
        format!("[{address}]:{port}")
    } else {
        format!("[{address}%{scope}]:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_native_table_counts_outside_the_buffer() {
        let result = table::<MIB_TCPROW_OWNER_PID>(4, |buffer, size| {
            if let Some(buffer) = buffer {
                // SAFETY: table allocated at least one u64 and requests a DWORD.
                unsafe {
                    buffer.cast::<u32>().write(999);
                }
                *size = 4;
                0
            } else {
                *size = 8;
                122
            }
        });
        assert!(result.is_err());
    }
}
