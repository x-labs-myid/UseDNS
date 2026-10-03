# UseDNS code audit — 2026-10-03

The application is a Rust/Slint desktop utility. `main.rs` connects UI callbacks and workers, `system.rs` handles Windows DNS and UAC, `storage.rs` persists settings, `models.rs` defines providers, and `tray.rs` creates the native tray menu. The Vue project under `docs/` is a separate documentation site. This change introduces `hover.rs`, `startup.rs`, `booster.rs`, and `native_windows.rs`. The Windows backend dependency uses the same version as Slint.

## Findings addressed

| Finding | Change |
| --- | --- |
| Close immediately hid the window without a choice | Both titlebar close and native close/Alt+F4 now show Quit, Background, and Cancel. Background is disabled without a tray. Quit/background wait for an active DNS write to finish. |
| Preview depended on tray Leave events and ignored preview hover | Poll native cursor and window bounds every 50 ms while visible; a 150 ms grace period allows crossing the gap. Hides without relying on Leave events. |
| Tray Move repeatedly showed/repositioned the preview | Show and position only when hidden. Clamp to the tray monitor; support taskbars at the top and monitors with negative coordinates. |
| Hovering the tray created a taskbar button for the preview | Configure winit's skip-taskbar and non-activating attributes before native window creation. Maintain tool-window/non-activating styles across preview events and show/hide cycles; keep the main window's normal taskbar behavior. |
| DNS profile modal reappeared after leaving and returning to Providers | Route every navigation action through one synchronous callback that closes the profile modal, exit confirmation, and navigation dropdown before changing pages. Includes Home/Policy shortcuts, custom-provider actions, and booster review; explicitly opening a fresh modal after navigation remains supported. |
| Large traffic readings stayed in Kbps and used a manually forced unit | Share adaptive decimal Kbps/Mbps/Gbps formatting across Download, Upload, total, graph, and tray. Floor compact Kbps/Mbps values to avoid crossing the display boundary by rounding. Remove manual selection and obsolete persistence; legacy settings remain readable. |
| Text-only DNS status concealed how response latency was judged | Show 1–5 stars in Home and tray using explicit latency bands (≤50/100/250/500/>500 ms). Failed/unmeasured probes remain unrated. Document that thresholds are application guidance and UDP latency is not DoH or overall internet quality. |
| Feature behavior lacked in-app explanations | Add an offline bilingual Guide & FAQ page with topic filters and expandable answers; include the sixth page in central navigation and regression checks. Sampling interval text now follows the actual preference. |
| Preview data could become stale while the cursor stayed over it | Refresh live DNS, status, latency, and theme while visible. |
| Tray menu remained stale after language/profile edits | Rebuild the menu only when its revision changes, preserving the existing tray icon. |
| Tray polling allocated three Strings every 50 ms | Compare Slint shared strings; update checkmarks only when state changes. |
| Full-size application PNG used for the tiny tray icon | Resize once to 32 × 32 pixels during creation. |
| No Windows startup setting | Off-by-default per-user Run entry with a quoted executable path and `--background`; read actual registration and roll back if saving fails. |
| No measured mechanism for a booster | Bounded background DNS response benchmark with explicit review/apply. Reliability precedes latency; duplicate addresses are skipped. |
| Truncated/unrelated DNS packets could be mistaken for success | Validate transaction, flags, question, answer records, data bounds, and the presence of an A record. A connected UDP socket restricts the source. |
| Some preference writes silently discarded errors | Report persistence failures in the UI. |
| Failed custom profile saves/deletes changed live state first | Persist a proposed copy before committing it to the live model. |
| Corrupt settings discarded valid backup data | Retain the previous good settings and recover from missing/corrupt primary files. |
| Stored custom profiles could duplicate built-in IDs | Validate profiles, reject duplicate/empty IDs, and mark accepted entries custom. |
| DNS requests could overlap through different callbacks | Guard apply/reset/tray actions while a write or adapter refresh is in progress. |
| Applying through tray or the legacy callback left adapter cache stale | Update cached DNS, automatic mode, and encryption state on success. |
| Hidden main window still updated graph rows | Continue sampling for tray speed, but skip graph updates while hidden or away from Home. |
| Missing adapter left speed baseline intact | Reset sampling baseline and displayed upload/download when no adapter is selected. |
| Reading Windows theme launched a process on the UI thread | Read the DWORD directly through the Windows registry API. |
| Minimize could fall back to an unrelated foreground window | Use only the application's own handle, or Slint's minimize API. |
| TCP reachability checks marked UDP-only router DNS as Unstable | Send and validate actual UDP DNS queries, try another domain/configured resolver, and measure the successful query. Show Not verified if no query succeeds rather than asserting an unstable internet connection. |
| Background DNS check raised a failed-operation toast and never recovered automatically | Update diagnostic state without an operation-error toast; retry every 15 seconds, immediately after DNS changes, and on refresh. Prevent concurrent checks and discard stale results. Localize state in Home and the tray, and keep it independent of other operation errors. |
| PowerShell could hang indefinitely | Drain stdout/stderr concurrently, enforce a 45-second deadline, kill/wait on timeout, and report failure. |
| Backend relied on the UI to validate addresses | Reject invalid IP addresses, empty adapter names, and non-HTTPS DoH templates before elevation. |

## Automated verification

Run `cargo fmt --all -- --check`, `cargo test --offline`, `cargo clippy --offline --all-targets -- -D warnings`, and `cargo build --offline` on Windows. Tests use a local UDP server, temporary settings files, and a hidden sleeping PowerShell process to verify timeout/termination; they do not change system DNS or startup registration.

The ignored native-window test must run on an isolated, inactive Windows desktop. It checks actual HWND styles across three preview show/hide cycles and verifies that the main window stays hidden until explicitly reopened. After `cargo test --offline`, run `python scripts/test-native-windows.py target/debug/deps/usedns-<test-hash>.exe`, using the test executable printed by Cargo. The runner creates an inactive desktop and never switches away from the user's desktop. Visual verification against Explorer's taskbar remains part of the manual checklist.

Run `python scripts/test-native-windows.py target/debug/deps/usedns-<test-hash>.exe native_windows::tests::navigation_closes_overlays_for_every_page` separately to check all 36 combinations of navigation between the six pages, including returning to the source page and deliberately opening a fresh modal after navigating (booster review).

Run `python scripts/test-native-windows.py target/debug/deps/usedns-<test-hash>.exe native_windows::tests::adaptive_metrics_and_documentation_render` to render review PNGs in `target/ui-review`. It exercises Home, navigation, the FAQ in both languages (including a pointer click to expand an answer), Settings, and tray ratings of 1, 5, and unmeasured. Unit tests cover conversion boundaries, each rating threshold, and migration of old manual-unit preferences.

## Manual Windows verification still required

- Close from the titlebar and Alt+F4: Cancel remains visible; Background preserves the tray; Quit terminates the application.
- Open the DNS profile modal, navigate to each other page through the dropdown, then return: the modal stays closed. Check Home/Policy shortcuts, custom-provider add/edit/save, and booster review as well.
- Attempt close while UAC/apply/reset is active: wait for completion or cancel the close dialog.
- Move rapidly away from the tray; move onto the preview; leave the preview; repeat through the hidden-icons overflow.
- With the main window hidden, hover the tray repeatedly: no taskbar button or focus change. Reopen the main window: its normal taskbar button returns. Repeat with the main window already open and after restarting Explorer.
- Check top/bottom/side taskbars, negative-coordinate monitors, and mixed DPI. The window must stay on the tray's monitor.
- Toggle startup on/off, check the current-user Run entry, and sign out/in. Startup must begin in the tray and fall back to a visible main window if tray creation fails.
- Run the booster on working DNS, blocked UDP DNS, a disconnected adapter, and a filtered/VPN network. Verify results do not alter DNS until Apply/UAC.
- Change language and add/edit/delete a custom provider: verify the tray menu updates and reflects the selected profile.

## Remaining limitations

- DNS writes remain Windows-specific. OS/version support for encrypted DNS must be validated on target Windows installations.
- DNS apply/reset are multi-step system operations; an error/timeout can leave a partial configuration. Full previous-state verification/rollback is a separate improvement.
- The booster samples three fixed domains over IPv4 UDP and does not evaluate IPv6, DoH, censorship, ISP interception, privacy, or protection quality. It is a small diagnostic, not a general throughput optimizer.
- Manual UI interactions, UAC, real sign-in behavior, installer lifecycle, and non-Windows builds are not covered by the unit suite.
- Multiple application instances can still be started. Cross-process locking/activation and coordination of settings writes would be useful follow-up work.
- The startup checkbox reports this executable's Run registration. Windows policy or Task Manager can separately disable execution. A moved/uninstalled executable requires the entry to be updated/removed.
- `main.rs` remains large; separating controller modules would improve maintainability, but is not by itself a proven runtime optimization.

## Mechanism references

- [Microsoft: Run and RunOnce registry keys](https://learn.microsoft.com/windows/win32/setupapi/run-and-runonce-registry-keys)
- [Microsoft: DNS queries and caching](https://learn.microsoft.com/en-us/windows-server/networking/dns/queries-lookups)
- [Microsoft: network adapter/TCP tuning](https://learn.microsoft.com/en-us/windows-server/networking/technologies/network-subsystem/net-sub-performance-tuning-nics)
