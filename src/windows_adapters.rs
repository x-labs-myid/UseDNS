//! Read-only adapter discovery without PowerShell/CIM startup dependencies.
use crate::system::AdapterInfo;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use windows::{
    Win32::{
        Foundation::HANDLE,
        NetworkManagement::IpHelper::{
            GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_MULTICAST,
            GetAdaptersAddresses, GetBestInterfaceEx, IP_ADAPTER_ADDRESSES_LH,
        },
        NetworkManagement::WiFi::{
            DOT11_SSID, WLAN_CONNECTION_ATTRIBUTES, WLAN_INTERFACE_INFO_LIST, WlanCloseHandle,
            WlanEnumInterfaces, WlanFreeMemory, WlanOpenHandle, WlanQueryInterface,
            wlan_interface_state_connected, wlan_intf_opcode_current_connection,
        },
        Networking::NetworkListManager::{INetworkListManager, NetworkListManager},
        Networking::WinSock::{AF_INET, AF_INET6, SOCKADDR_IN, SOCKADDR_IN6, SOCKET_ADDRESS},
        System::Com::{
            CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
        },
        System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_QWORD, RRF_RT_REG_SZ, RegGetValueW},
    },
    core::{PCWSTR, PWSTR},
};

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn registry_string(path: &str, value: &str) -> String {
    let path = wide(path);
    let value = wide(value);
    let mut bytes = 0;
    // SAFETY: NUL-terminated strings; the second call writes into an allocated
    // UTF-16 buffer sized by the first call. These calls only read HKLM.
    unsafe {
        if RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(path.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut bytes),
        )
        .is_err()
        {
            return String::new();
        }
        let mut buffer = vec![0u16; (bytes as usize).div_ceil(2)];
        if RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(path.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
        .is_err()
        {
            return String::new();
        }
        let end = buffer.iter().position(|v| *v == 0).unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end])
    }
}

fn encrypted(path: &str) -> bool {
    let path = wide(path);
    let value = wide("DohFlags");
    let mut flags = 0u64;
    let mut bytes = std::mem::size_of::<u64>() as u32;
    // SAFETY: RegGetValueW is restricted to QWORD and an eight-byte buffer.
    unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(path.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_QWORD,
            None,
            Some((&mut flags as *mut u64).cast()),
            Some(&mut bytes),
        )
        .is_ok()
            && flags == 17
    }
}

unsafe fn socket_ip(address: &SOCKET_ADDRESS) -> Option<IpAddr> {
    if address.lpSockaddr.is_null() || address.iSockaddrLength < std::mem::size_of::<u16>() as i32 {
        return None;
    }
    // SAFETY: Family and length are checked before reading the matching native
    // socket-address structure, owned by the GetAdaptersAddresses buffer.
    unsafe {
        match (*address.lpSockaddr).sa_family {
            AF_INET if address.iSockaddrLength >= std::mem::size_of::<SOCKADDR_IN>() as i32 => {
                let address = address.lpSockaddr.cast::<SOCKADDR_IN>().read_unaligned();
                Some(Ipv4Addr::from(address.sin_addr.S_un.S_addr.to_ne_bytes()).into())
            }
            AF_INET6 if address.iSockaddrLength >= std::mem::size_of::<SOCKADDR_IN6>() as i32 => {
                let address = address.lpSockaddr.cast::<SOCKADDR_IN6>().read_unaligned();
                Some(Ipv6Addr::from(address.sin6_addr.u.Byte).into())
            }
            _ => None,
        }
    }
}

struct WifiHandle(HANDLE);
impl Drop for WifiHandle {
    fn drop(&mut self) {
        // SAFETY: This handle is owned after a successful WlanOpenHandle call.
        unsafe {
            WlanCloseHandle(self.0, None);
        }
    }
}
struct WifiMemory(*mut std::ffi::c_void);
impl Drop for WifiMemory {
    fn drop(&mut self) {
        // SAFETY: WlanEnumInterfaces/WlanQueryInterface allocate this memory.
        unsafe {
            WlanFreeMemory(self.0);
        }
    }
}
fn ssid_name(ssid: &DOT11_SSID) -> Option<String> {
    let len = ssid.uSSIDLength as usize;
    if len == 0 || len > ssid.ucSSID.len() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&ssid.ucSSID[..len])
            .chars()
            .map(|c| if c.is_control() { '\u{fffd}' } else { c })
            .collect(),
    )
}
fn wifi_networks() -> Vec<(String, String)> {
    let mut networks = Vec::new();
    let mut handle = HANDLE::default();
    let mut version = 0;
    // SAFETY: All output pointers refer to live initialized local variables.
    unsafe {
        if WlanOpenHandle(2, None, &mut version, &mut handle) != 0 {
            return networks;
        }
        let handle = WifiHandle(handle);
        let mut interfaces: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        if WlanEnumInterfaces(handle.0, None, &mut interfaces) != 0 || interfaces.is_null() {
            return networks;
        }
        let _interfaces = WifiMemory(interfaces.cast());
        let list = &*interfaces;
        // The native allocation includes dwNumberOfItems entries after its header.
        for index in 0..list.dwNumberOfItems as usize {
            let interface = list.InterfaceInfo.as_ptr().add(index).read();
            if interface.isState != wlan_interface_state_connected {
                continue;
            }
            let mut size = 0;
            let mut data: *mut std::ffi::c_void = std::ptr::null_mut();
            if WlanQueryInterface(
                handle.0,
                &interface.InterfaceGuid,
                wlan_intf_opcode_current_connection,
                None,
                &mut size,
                &mut data,
                None,
            ) != 0
                || data.is_null()
            {
                continue;
            }
            let _data = WifiMemory(data);
            if (size as usize) < std::mem::size_of::<WLAN_CONNECTION_ATTRIBUTES>() {
                continue;
            }
            let connection = data.cast::<WLAN_CONNECTION_ATTRIBUTES>().read();
            if connection.isState != wlan_interface_state_connected {
                continue;
            }
            if let Some(ssid) = ssid_name(&connection.wlanAssociationAttributes.dot11Ssid) {
                let guid = format!("{:?}", interface.InterfaceGuid)
                    .trim_matches(['{', '}'])
                    .to_ascii_lowercase();
                networks.push((guid, ssid));
            }
        }
    }
    networks
}

struct ComApartment(bool);
impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: Balance our successful CoInitializeEx on the same thread,
            // after every local COM interface has been released.
            unsafe {
                CoUninitialize();
            }
        }
    }
}
fn connected_network_names() -> Vec<(String, String)> {
    let mut names = Vec::new();
    // SAFETY: Use COM only on this worker thread. Returned interface wrappers
    // own their references and are dropped before the apartment guard.
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_MULTITHREADED);
        if initialized.is_err() && initialized.0 != 0x80010106u32 as i32 {
            return names;
        }
        let _apartment = ComApartment(initialized.is_ok());
        let Ok(manager): Result<INetworkListManager, _> =
            CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL)
        else {
            return names;
        };
        let Ok(connections) = manager.GetNetworkConnections() else {
            return names;
        };
        loop {
            let mut item = [None];
            let mut fetched = 0;
            if connections.Next(&mut item, Some(&mut fetched)).is_err() || fetched == 0 {
                break;
            }
            let Some(connection) = item[0].take() else {
                break;
            };
            if !connection.IsConnected().is_ok_and(|value| value.0 != 0) {
                continue;
            }
            let (Ok(guid), Ok(network)) = (connection.GetAdapterId(), connection.GetNetwork())
            else {
                continue;
            };
            if let Ok(name) = network.GetName() {
                let name = name.to_string();
                if !name.is_empty() {
                    names.push((format!("{guid:?}").to_ascii_lowercase(), name));
                }
            }
        }
    }
    names
}

pub fn active_adapters() -> Result<Vec<AdapterInfo>, String> {
    let wifi = wifi_networks();
    let mut network_names: Option<Vec<(String, String)>> = None;
    let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST;
    let mut size = 15_000u32;
    for _ in 0..4 {
        // u64 storage provides the alignment required by the native structures.
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        size = (buffer.len() * 8) as u32;
        // SAFETY: The buffer is writable, aligned and lives until all linked
        // adapter/DNS nodes and strings have been copied below. AF_UNSPEC=0.
        let status = unsafe {
            GetAdaptersAddresses(0, flags, None, Some(buffer.as_mut_ptr().cast()), &mut size)
        };
        if status == 111 {
            continue;
        }
        if status == 232 {
            return Err("No active network adapter was found.".into());
        }
        if status != 0 {
            return Err(format!(
                "Could not read network adapters (Windows error {status})."
            ));
        }
        let mut best_index = 0;
        let mut destination = SOCKADDR_IN {
            sin_family: AF_INET,
            ..Default::default()
        };
        destination.sin_addr.S_un.S_addr = u32::from_ne_bytes([1, 1, 1, 1]);
        // SAFETY: Only queries the local routing table; no packet is sent.
        let _ = unsafe {
            GetBestInterfaceEx((&destination as *const SOCKADDR_IN).cast(), &mut best_index)
        };
        let mut node = buffer.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let mut result = Vec::new();
        // SAFETY: Successful GetAdaptersAddresses returns a valid linked list
        // in this live buffer. All strings and addresses are copied, not retained.
        unsafe {
            while let Some(adapter) = node.as_ref() {
                node = adapter.Next;
                if adapter.OperStatus.0 != 1 || adapter.IfType == 24 {
                    continue;
                }
                let name = PWSTR(adapter.FriendlyName.0)
                    .to_string()
                    .unwrap_or_default();
                if name.is_empty() {
                    continue;
                }
                let guid = adapter.AdapterName.to_string().unwrap_or_default();
                let mut addresses = Vec::new();
                let mut dns = adapter.FirstDnsServerAddress;
                while let Some(server) = dns.as_ref() {
                    if let Some(ip) = socket_ip(&server.Address)
                        && !addresses.contains(&ip)
                    {
                        addresses.push(ip);
                    }
                    dns = server.Next;
                }
                let index = adapter.Anonymous1.Anonymous.IfIndex;
                let index = if index == 0 {
                    adapter.Ipv6IfIndex
                } else {
                    index
                };
                let v4 = registry_string(
                    &format!(
                        "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces\\{guid}"
                    ),
                    "NameServer",
                );
                let v6 = registry_string(
                    &format!(
                        "SYSTEM\\CurrentControlSet\\Services\\Tcpip6\\Parameters\\Interfaces\\{guid}"
                    ),
                    "NameServer",
                );
                let encrypted = addresses.iter().any(|ip| {
                    let family = if ip.is_ipv4() { "Doh" } else { "Doh6" };
                    [guid.as_str(), guid.trim_matches(['{', '}'])].iter().any(|guid| encrypted(&format!("SYSTEM\\CurrentControlSet\\Services\\Dnscache\\InterfaceSpecificParameters\\{guid}\\DohInterfaceSettings\\{family}\\{ip}")))
                });
                let description = adapter
                    .Description
                    .to_string()
                    .unwrap_or_default()
                    .to_lowercase();
                let virtual_adapter = ["virtual", "hyper-v", "loopback", "tap", "vpn"]
                    .iter()
                    .any(|term| description.contains(term));
                let rank = if index == best_index {
                    0
                } else if virtual_adapter {
                    2
                } else {
                    1
                };
                result.push((
                    (
                        rank,
                        adapter.FirstGatewayAddress.is_null(),
                        adapter.Ipv4Metric,
                    ),
                    AdapterInfo {
                        label: if adapter.IfType == 71 {
                            let networks = if wifi.iter().any(|(id, _)| {
                                guid.trim_matches(['{', '}']).eq_ignore_ascii_case(id)
                            }) {
                                &wifi
                            } else {
                                &*network_names.get_or_insert_with(connected_network_names)
                            };
                            networks
                                .iter()
                                .find(|(id, _)| {
                                    guid.trim_matches(['{', '}']).eq_ignore_ascii_case(id)
                                })
                                .map(|(_, ssid)| format!("{ssid} \u{00b7} {name}"))
                                .unwrap_or_else(|| name.clone())
                        } else {
                            name.clone()
                        },
                        name,
                        interface_index: index,
                        dns: addresses
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", "),
                        dns_automatic: v4.trim().is_empty() && v6.trim().is_empty(),
                        dns_encrypted: encrypted,
                    },
                ));
            }
        }
        result.sort_by_key(|(rank, _)| *rank);
        return if result.is_empty() {
            Err("No active network adapter was found.".into())
        } else {
            Ok(result.into_iter().map(|(_, adapter)| adapter).collect())
        };
    }
    Err("Network adapter configuration changed repeatedly. Refresh to retry.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ssid_uses_byte_length_and_preserves_unicode() {
        let mut ssid = DOT11_SSID {
            uSSIDLength: 3,
            ucSSID: [b'x'; 32],
        };
        ssid.ucSSID[..3].copy_from_slice("Caf".as_bytes());
        assert_eq!(ssid_name(&ssid).as_deref(), Some("Caf"));
        let text = "Caf\u{e9} Wi-Fi";
        ssid.uSSIDLength = text.len() as u32;
        ssid.ucSSID[..text.len()].copy_from_slice(text.as_bytes());
        assert_eq!(ssid_name(&ssid).as_deref(), Some(text));
        ssid.uSSIDLength = 33;
        assert!(ssid_name(&ssid).is_none());
        ssid.uSSIDLength = 0;
        assert!(ssid_name(&ssid).is_none());
    }
    #[test]
    fn connected_network_names_remain_scoped_to_adapter_guids() {
        for (guid, name) in connected_network_names() {
            assert_eq!(guid.len(), 36);
            assert!(!name.is_empty());
            assert_eq!(guid.chars().filter(|ch| *ch == '-').count(), 4);
        }
    }
    #[test]
    fn reads_local_adapters_without_powershell_or_cim() {
        match active_adapters() {
            Ok(adapters) => {
                assert!(!adapters.is_empty());
                for adapter in adapters {
                    assert!(!adapter.name.is_empty());
                    assert_ne!(adapter.interface_index, 0);
                    assert!(
                        adapter
                            .dns
                            .split(", ")
                            .filter(|s| !s.is_empty())
                            .all(|s| s.parse::<IpAddr>().is_ok())
                    );
                }
            }
            Err(error) => assert_eq!(error, "No active network adapter was found."),
        }
    }
}
