# Network tools and platform DNS

The Windows DNS apply/reset scripts are retained. The elevated helper has one additional operation for restarting WinNAT; it does not run automatically after DNS or port actions. Existing settings and stored provider schemas are unchanged.

## Platform support

| Capability | Windows | Linux | macOS |
| --- | --- | --- | --- |
| DNS discovery/apply/reset | Native IP Helper discovery; existing PowerShell/UAC writes | NetworkManager, nmcli, pkexec | networksetup, scoped scutil DNS, osascript authorization |
| Encrypted system DNS | Existing behavior | Unavailable; plain DNS only | Unavailable; plain DNS only |
| Speedtest | libcurl / Schannel | libcurl / system TLS | libcurl / system TLS |
| Port inspection | Native IP Helper TCP/UDP tables | lsof and ps | lsof and ps |
| Normal / force termination | taskkill / taskkill /F | SIGTERM / SIGKILL | SIGTERM / SIGKILL |
| WinNAT restart | Separate confirmed UAC action | Hidden | Hidden |

Linux without NetworkManager is unsupported. Authorization requires a graphical polkit agent. Existing Windows-only live adapter throughput, startup, and tray behavior are unchanged; these additions do not implement Unix equivalents for them.

## Behavior and limits

All tools run on explicit action, on worker threads. Speedtest uses libcurl to contact Cloudflare, with four reusable parallel sessions per direction, a 3-second warmup excluded from the result, and 10 seconds of measured transfer. Request sizes adapt to observed throughput (64 KB to 32 MB). Download bodies are discarded and uploads use a bounded generated payload, without temporary files. Live values use approximately one second of aggregate byte deltas; final throughput uses all measured bytes over wall-clock time, including partial requests at the measurement boundary. Data usage scales with connection speed and can reach gigabytes. HTTP response latency is the median of six samples after one discarded warmup request, subtracting pretransfer setup time; it includes server processing and is not ICMP ping. Cancellation and errors stop all sessions; connection and request timeouts are 5 and 20 seconds. TLS verification stays enabled. Results stay in memory and do not claim bandwidth gains from DNS changes. This is not the Ookla algorithm or the complete official Cloudflare speedtest engine.

Opening Network Tools automatically requests local port data in the background. Ports include local TCP listeners and UDP endpoints. Process ownership may be hidden by OS permissions. Normal stop and force stop each require confirmation, including the selected process details. Force stop has an additional unsaved-work warning. Termination rechecks the socket owner and process start time, and rejects unavailable identity, protected low PIDs, and UseDNS itself. Unix process start times have one-second precision, so this reduces but cannot completely eliminate process races. Termination does not elevate or terminate descendants automatically.

WinNAT confirmation explains disruption to VM/container connections. DNS writes and WinNAT restart share the busy guard. Restart stops then starts the service and attempts to start it again if the first start fails. Missing services, cancellation, and permission errors are reported. No reserved port ranges are deleted.

Linux snapshots both families' DNS profile fields before modification and attempts rollback if device reapply fails. Failed rollback is reported. macOS applies the selected service DNS through networksetup. Neither backend restarts networking or edits resolv.conf directly.

## Verification

Automated regression coverage includes existing DNS, models, settings recovery, monitoring units, booster, and tray hover behavior, plus port parsing, protected-process guards, cancellation, Windows enumeration of actual local sockets, Unix request validation, and scoped macOS resolver parsing. The isolated native runner renders Network Tools in both languages and themes and checks all 49 navigation transitions.

The check workflow builds/tests/lints Windows, Linux, and macOS after pushing a branch or opening a pull request. Local Windows tests cannot establish live Linux/macOS DNS compatibility. On each supported OS, manually validate:

1. Discover the active service/device and compare DNS with OS settings, including IPv4-only and IPv6-only connections.
2. Apply plain IPv4, IPv6, and dual-stack DNS; compare the resulting OS configuration. Cancel authorization and verify that no changes occurred.
3. Reset to automatic DNS. On Linux, simulate failed reapply and check rollback/error reporting.
4. Test and cancel speedtest, including offline and slow connections.
5. Start a disposable local server, refresh ports, normally stop it, and verify the port disappears. Repeat force stop only with a disposable process. Check that unrelated processes stay alive.
6. On Windows, restart WinNAT only when interrupting VM/container connections is acceptable, and verify the service returns to running.

### Local verification (2026-10-07)

- Windows: 22 active unit/integration tests passed, including real local socket enumeration and native adapter discovery without PowerShell.
- All four isolated native UI tests passed: empty-page/error rendering, screenshot rendering, 49 navigation combinations, and preview/taskbar behavior. Screenshot review includes normal stop, force stop, and WinNAT confirmations.
- Existing Windows DNS apply/reset scripts are unchanged. No live DNS configuration, process termination, or WinNAT restart was performed on the host.
- Live speedtest endpoint probing was blocked by the sandbox's Schannel credential context (SEC_E_NO_CREDENTIALS). TLS verification remains enabled; full internet throughput measurement still requires testing outside this sandbox.
- Linux/macOS code paths were type-checked through the Windows test build where available; live platform authorization and network configuration have not been tested locally. The CI matrix is configured but has not been run remotely in this session.

### Startup and empty-page fix

Windows adapter discovery now uses GetAdaptersAddresses, the local routing table, and read-only registry queries, avoiding the startup PowerShell/CIM timeout. DNS apply/reset retain their existing elevated scripts. Windows retains its original default renderer to preserve transparent rounded window corners. Software rendering is used only by explicitly isolated UI checks. Network Tools includes an explicit empty-port hint and a minimum scroll viewport height. Visual checks cover empty data during refresh, after a simulated discovery error, and after requesting a smaller window. The isolated desktop retained the 1180 by 800 capture size, so the requested 830 by 500 size was not visually verified.

### Tool tabs and presentation

Network Tools separates Speedtest, Ports & Processes, and Windows-only WinNAT into tabs. Speedtest renders structured download/upload/HTTP latency results in theme-aware cards, with a cancellable running state and an inline error. Switching tabs clears pending destructive confirmations without cancelling background work. Port scanning, process identity checks, and elevated WinNAT restart retain their existing behavior.

### Speedtest activity

An animated activity ring and three stage segments identify latency, download, and upload while testing. Elapsed time and a spinner in the active metric remain visible. The ring indicates activity; its center and metric cards show measured values rather than fabricated readings or a time-based completion percentage. Live transfer values are delivered every 250 ms after warmup, retaining partial values on cancellation or failure while clearly marking the test interrupted. Timers stop when the activity panel is hidden. TLS verification stays enabled.

### Rounded window restoration (2026-10-08)

Removed the production software-renderer override and restored the original default backend. The existing transparent window, 20 px frame radius, rounded clipping, and Windows corner preference remain in place. Existing functional tests and lint passed. GPU screenshot readback on the isolated inactive Windows desktop returned an empty framebuffer, so the restored GPU frame was not visually verified in that environment.

### Connected Wi-Fi labels

Windows reads SSIDs via the Native Wi-Fi API. If unavailable, it uses the connected Windows network name from Network List Manager, matching the adapter GUID and excluding disconnected profiles. That fallback is a network display name, which can differ from an SSID if renamed. Only the display label changes; DNS commands retain the adapter alias/index. Ethernet labels and the rounded application frame are unchanged. Unavailable network names fall back to the adapter name.

### Duration-based speedtest update

The existing rounded layout and renderer are retained. Download/upload cards receive measured byte progress every 250 ms after warmup; the activity ring remains a phase indicator. Cross-platform libcurl replaces the speedtest-only curl subprocess dependency; port and DNS commands remain unchanged.
