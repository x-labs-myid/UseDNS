# UseDNS MVP — Product and Technical Specification

**Document status:** Implemented MVP baseline  
**Application version:** `0.1.0`  
**Primary platform:** Windows 10 and Windows 11  
**UI framework:** Slint `1.17.1`  
**Implementation language:** Rust 2024 edition  
**Default language:** English  
**License:** MIT

---

## 1. Executive Summary

UseDNS is a lightweight desktop utility for viewing, comparing, applying, and restoring DNS configurations on Windows network adapters. The application is intended to remove the need for users to manually navigate Windows network settings or memorize DNS addresses.

The MVP combines four primary capabilities:

1. Display the active network adapter, current DNS addresses, connection reachability, latency, and real-time adapter throughput.
2. Present a curated, read-only catalog of public DNS providers and apply their IPv4, IPv6, or combined configurations.
3. Allow users to create, edit, validate, test, persist, and delete custom DNS profiles.
4. Explain the application's privacy behavior and provide a bilingual, theme-aware desktop interface.

UseDNS does not operate a DNS resolver. DNS queries are sent directly by Windows to the resolver selected by the user.

---

## 2. MVP Goals

### 2.1 Product goals

- Make DNS configuration understandable for non-technical users.
- Reduce the number of steps required to switch DNS providers.
- Explain the purpose, advantages, and trade-offs of bundled providers.
- Support advanced configurations through custom IPv4 and IPv6 profiles.
- Always provide a path back to automatic DNS supplied by DHCP.
- Keep user preferences and custom DNS data local to the device.
- Provide a coherent English and Indonesian experience.
- Offer a minimal, glass-inspired interface based on the UseDNS logo palette.

### 2.2 Technical goals

- Keep UI presentation separated from domain and platform operations.
- Keep privileged DNS operations outside Slint callbacks and the UI thread.
- Validate addresses before storing or applying custom profiles.
- Preserve compatibility when new settings fields are introduced.
- Keep bundled DNS profiles immutable.
- Make future Linux and macOS platform backends possible without redesigning the entire UI.

### 2.3 Non-goals for the current MVP

- Operating a proprietary DNS resolver.
- Capturing or inspecting DNS queries.
- Claiming anonymity after a DNS change.
- Performing a full internet bandwidth speed test by downloading remote test files.
- Automatic privilege elevation through a Windows UAC relaunch flow.
- System tray support.
- Automatic rollback to the exact previous static DNS configuration.
- Native DNS-changing backends for Linux or macOS.
- DNS-over-HTTPS or DNS-over-TLS configuration.
- Account synchronization or cloud storage.

---

## 3. Implemented Feature Inventory

## 3.1 Home / Network Status

The Home page is implemented in `ui/pages/home.slint`.

### Current behavior

- Shows a welcome header and UseDNS branding.
- Lists active Windows network adapters in an adapter selector.
- Shows whether an adapter is available and the application considers the connection connected or disrupted.
- Displays the current DNS server addresses returned by Windows.
- Displays the active provider label.
  - Defaults to `System / DHCP` at startup.
  - Changes to the selected provider name after a successful apply operation in the current session.
- Checks resolver reachability using the operating system's `ping` command.
- Displays approximate latency in milliseconds.
- Classifies connection quality as:
  - `Stable` below 100 ms.
  - `Fair` from 100 ms up to 249 ms.
  - `Slow` at 250 ms or above.
  - `Unstable` when the resolver does not respond.
- Provides a manual Refresh action.
- Provides navigation to the DNS provider catalog.
- Provides an action to restore automatic DNS through DHCP.

### Real-time network activity

- Reads Windows network-adapter byte counters through `Get-NetAdapterStatistics`.
- Samples received and sent byte counters every second.
- Converts the counter delta into megabits per second.
- Displays:
  - Current download activity.
  - Current upload activity.
  - Relative progress indicators.
- Sampling runs outside the Slint UI thread.
- An atomic guard prevents overlapping statistics requests.
- Counter history is reset logically when the selected adapter changes.

### Important interpretation

The displayed Mbps values represent current adapter traffic, not maximum line speed. This is not a traditional bandwidth speed test. If the device is idle, values near `0 Mbps` are expected.

---

## 3.2 DNS Provider Catalog

The provider page is implemented in `ui/pages/providers.slint`.

### Included providers

| ID | Provider | IPv4 | IPv6 | Primary focus |
| --- | --- | --- | --- | --- |
| `cloudflare` | Cloudflare | `1.1.1.1`, `1.0.0.1` | `2606:4700:4700::1111`, `2606:4700:4700::1001` | Performance and privacy |
| `google` | Google Public DNS | `8.8.8.8`, `8.8.4.4` | `2001:4860:4860::8888`, `2001:4860:4860::8844` | Reliability and availability |
| `quad9` | Quad9 | `9.9.9.9`, `149.112.112.112` | `2620:fe::fe`, `2620:fe::9` | Malware and phishing protection |
| `adguard` | AdGuard DNS | `94.140.14.14`, `94.140.15.15` | `2a10:50c0::ad1:ff`, `2a10:50c0::ad2:ff` | Ad and tracker filtering |

### Catalog presentation

Each provider card can display:

- Provider monogram.
- Provider name.
- Localized description.
- IPv4 addresses.
- IPv6 addresses.
- Focus/category badge.
- Active badge when an address matches the detected DNS string.
- Advantages.
- Trade-offs or limitations.
- Custom badge for user-created profiles.

### Search

Search is implemented in Rust and matches case-insensitively against:

- Provider name.
- English summary.
- Indonesian summary.

The result model is rebuilt and sent to Slint after the search input changes.

### Category filters

| Filter | Included bundled providers |
| --- | --- |
| All | Every bundled and custom provider |
| Privacy | Cloudflare and Quad9 |
| Speed | Cloudflare and Google Public DNS |
| Security | Quad9 and AdGuard DNS |
| Ad blocking | AdGuard DNS |

Custom providers remain discoverable under All and through search. More detailed custom-category integration is planned.

### Detail expansion

- A provider card can be expanded and collapsed.
- Expanded details show advantages and limitations.
- Custom entries expose Edit and Delete actions.
- Bundled entries never expose Edit or Delete actions.

### Address mode

Before applying a provider, the user can choose:

- IPv4 only.
- IPv6 only.
- IPv4 and IPv6 together.

The backend omits blank secondary addresses. Applying a mode for which the provider has no address produces an error instead of silently applying an incomplete configuration.

---

## 3.3 Applying DNS

DNS operations are implemented in `src/system.rs` and orchestrated from `src/main.rs`.

### Windows command

UseDNS applies DNS with:

```powershell
Set-DnsClientServerAddress -InterfaceAlias '<adapter>' -ServerAddresses (...)
```

### Apply flow

1. User selects an adapter.
2. User selects IPv4, IPv6, or combined mode.
3. User selects Use DNS on a provider card.
4. Rust resolves the provider by stable ID.
5. Rust resolves the adapter by the selected index.
6. Empty addresses are removed.
7. The DNS operation runs on a worker thread.
8. Slint shows a busy overlay during the operation.
9. On success:
   - The active provider label is updated.
   - The current DNS display is updated.
   - A localized success message is shown.
   - Network status is refreshed.
10. On failure, the PowerShell or platform error is shown in the application.

### Privilege requirement

Applying DNS requires Administrator privileges. The MVP does not automatically elevate itself. Users must run the terminal or executable as Administrator.

---

## 3.4 Restore Automatic DNS

UseDNS can restore DNS server addresses supplied by DHCP.

### Windows command

```powershell
Set-DnsClientServerAddress -InterfaceAlias '<adapter>' -ResetServerAddresses
```

### Available locations

- Home page.
- Policy page.

### Result

- On success, the active provider label returns to `System / DHCP`.
- The adapter state is refreshed.
- A localized result message is displayed.

This operation also requires Administrator privileges.

---

## 3.5 Custom DNS Management

The custom DNS page is implemented in `ui/pages/custom-dns.slint`.

### Layout

- Header with selected adapter and connectivity status.
- Informational banner explaining appropriate custom-DNS use cases.
- Scrollable form area on the left.
- Live Preview and Status panel on the right.
- Dedicated Save, Reset, and Test Resolver actions.

### Editable fields

- DNS profile name.
- Short description.
- IPv4 primary address.
- IPv4 secondary address.
- IPv6 primary address.
- IPv6 secondary address.
- Purpose/category.

### Supported custom purposes

- `general`
- `home`
- `office`
- `pihole`
- `privacy`
- `local`

The purpose is serialized with the custom provider and restored during editing. Older stored profiles without this field remain compatible through Serde defaults.

### Validation rules

Implemented in `DnsProvider::validate`:

- Name must not be empty.
- At least one primary IPv4 or IPv6 address is required.
- Every non-empty address must parse as a valid IP address.
- IPv4 fields reject IPv6 addresses.
- IPv6 fields reject IPv4 addresses.
- A secondary address is not accepted without its corresponding primary address.

### Create flow

- New custom profiles receive an ID based on the current Unix timestamp in milliseconds.
- The profile is validated.
- The profile is appended to the provider collection.
- Custom profiles and preferences are written to local JSON storage.
- The application returns to the provider catalog.

### Edit flow

- Only providers marked `custom` can be loaded into the edit form.
- The existing stable ID is retained.
- Saving replaces the matching custom provider.
- Bundled providers cannot be loaded into this flow.

### Delete flow

- Only matching custom providers are removed.
- Bundled providers are retained even if an unexpected ID is supplied.
- Updated custom-provider data is persisted immediately.

### Test Resolver

- The Test Resolver action prefers the primary IPv4 address.
- If IPv4 is empty, it uses the primary IPv6 address.
- If both are empty, the UI reports that an address is required.
- The ping check runs on a worker thread.
- A successful test reports approximate latency.
- A failed test reports that the resolver could not be reached.

A successful ping proves reachability only. It does not prove that the resolver will correctly answer every DNS query.

---

## 3.6 Privacy and Policy

The policy page is implemented in `ui/pages/policy.slint`.

### Policy sections

1. **Application privacy**
   - UseDNS does not run a DNS resolver.
   - UseDNS does not read, record, or sell DNS queries.
   - Custom DNS, language, and theme data remain local.

2. **When DNS is applied**
   - Addresses are written to the active Windows adapter.
   - Apply and reset actions require Administrator rights.

3. **Third-party DNS providers**
   - Queries are processed directly by the selected provider.
   - Provider privacy, logging, filtering, and retention policies differ.

4. **Checks and security**
   - Resolver checks use ping and are not bandwidth tests.
   - DNS changes can affect access to internet or internal services.
   - Automatic DNS is available as a recovery action.

### Policy actions

- Navigate to DNS providers.
- Restore automatic DNS on the selected adapter.

---

## 3.7 Settings

The settings page is implemented in `ui/pages/settings.slint`.

### Appearance modes

- `system`
- `light`
- `dark`

### Follow System implementation

- Windows application-theme preference is read from:

```text
HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize
AppsUseLightTheme
```

- `0x0` is treated as dark mode.
- Other or unavailable values fall back to light mode.
- Slint's widget `Palette.color-scheme` is updated to match the effective mode.
- The custom UseDNS design tokens update at the same time.

The current MVP detects the system preference when the application starts and when System mode is selected. Continuous observation of Windows theme-change events is planned.

### Languages

- English (`en`) is the default.
- Indonesian (`id`) is available.
- Language changes apply immediately.
- Provider descriptions are rebuilt in the selected language.
- The preference is saved locally.

### Informational settings panels

The page accurately reports current MVP behavior:

- System tray startup is not available.
- Connection checking at startup is enabled.
- DNS operation results are displayed through in-app notifications.
- Adapter activity refreshes every second.
- Speed is displayed in Mbps.

These entries are informational in the current MVP and are not presented as configurable toggles.

---

## 3.8 Visual Design System

Design tokens are defined in `ui/theme.slint`.

### Visual direction

- Minimal, glass-inspired surfaces.
- Deep navy backgrounds in dark mode.
- Soft blue-white backgrounds in light mode.
- Royal blue primary actions.
- Cyan highlights and status accents.
- Semi-transparent surfaces.
- Thin borders.
- Rounded cards and controls.
- Palette derived from `.assets/UseDNS.png`.

### Adaptive tokens

The theme provides shared tokens for:

- Application background.
- Sidebar background.
- Standard surface.
- Strong/selected surface.
- Border.
- Primary text.
- Secondary text.
- Muted text.
- Primary accent.
- Cyan accent.
- Modal overlay.

All major pages consume these tokens rather than maintaining independent color palettes.

### Icon library standard

All UI icons across the application **must** use the Lucide icon set via the Slint port:
- **Porting for Slint:** [lucide-slint](https://github.com/cnlancehu/lucide-slint)
- **Original reference:** [Lucide Icons](https://github.com/lucide-icons/lucide)

Using ad-hoc ASCII/Unicode symbols, custom unstandardized SVG shapes, or inconsistent icon sets for standard UI iconography (navigation, actions, badges, indicators) is strictly prohibited. All icons must be integrated consistently using `lucide-slint` components and styled according to the design system tokens.

### Application icon

`.assets/UseDNS.png` is used for:

- Window icon.
- Sidebar branding.
- README branding.
- Windows executable resource generation.

At build time, `build.rs` generates icon frames at 16, 24, 32, 48, 64, 128, and 256 pixels and embeds them into the Windows executable.

---

## 4. Architecture

```text
UseDNS/
├── .assets/
│   ├── UseDNS.png
│   └── info.txt
├── docs/
│   └── spec/
│       └── MVP_SPECIFICATION.md
├── src/
│   ├── main.rs
│   ├── models.rs
│   ├── storage.rs
│   └── system.rs
├── ui/
│   ├── components/
│   │   ├── common.slint
│   │   └── sidebar.slint
│   ├── pages/
│   │   ├── custom-dns.slint
│   │   ├── home.slint
│   │   ├── policy.slint
│   │   ├── providers.slint
│   │   └── settings.slint
│   ├── app-window.slint
│   ├── theme.slint
│   └── types.slint
├── build.rs
├── Cargo.toml
├── Cargo.lock
├── LICENSE
└── README.md
```

## 4.1 `src/main.rs`

Responsibilities:

- Application composition root.
- Load persisted settings.
- Build Slint provider models.
- Connect Slint callbacks to Rust operations.
- Manage provider, language, theme, and adapter state.
- Dispatch slow or privileged operations to worker threads.
- Return results through `slint::invoke_from_event_loop`.
- Sample real-time network activity.
- Map backend results to UI state and notifications.

## 4.2 `src/models.rs`

Responsibilities:

- Define `DnsProvider`.
- Define bundled provider data.
- Validate IPv4 and IPv6 fields.
- Protect built-in/custom behavior through the `custom` marker.

## 4.3 `src/storage.rs`

Responsibilities:

- Define persistent `Settings`.
- Locate the platform application configuration directory.
- Load JSON settings with safe defaults.
- Save pretty-printed JSON settings.
- Preserve forward compatibility through field defaults.

## 4.4 `src/system.rs`

Responsibilities:

- Detect the Windows theme preference.
- Execute PowerShell safely without an interactive profile.
- Enumerate active adapters.
- Read effective DNS addresses.
- Apply DNS addresses.
- Reset DNS addresses.
- Read network byte counters.
- Ping a resolver for reachability and latency.
- Return explicit unsupported-platform errors outside Windows.

## 4.5 `ui/app-window.slint`

Responsibilities:

- Export the top-level `AppWindow` consumed by Rust.
- Own the shared UI properties and callback surface.
- Compose the sidebar and page components.
- Connect child-page callbacks to top-level callbacks.
- Display global busy and notification overlays.
- Synchronize the effective Slint widget color scheme.

## 4.6 UI modules

- `ui/theme.slint`: application-wide adaptive design tokens.
- `ui/types.slint`: UI-facing `ProviderRow` structure.
- `ui/components/common.slint`: reusable navigation, status, title, badge, and selection components.
- `ui/components/sidebar.slint`: branding and primary navigation.
- `ui/pages/home.slint`: network status and activity dashboard.
- `ui/pages/providers.slint`: search, filters, provider cards, and apply actions.
- `ui/pages/custom-dns.slint`: custom profile form, preview, purpose, and resolver testing.
- `ui/pages/policy.slint`: privacy explanations and recovery actions.
- `ui/pages/settings.slint`: appearance, language, and current-behavior information.

---

## 5. Data Model

## 5.1 DNS provider

```rust
pub struct DnsProvider {
    pub id: String,
    pub name: String,
    pub ipv4_primary: String,
    pub ipv4_secondary: String,
    pub ipv6_primary: String,
    pub ipv6_secondary: String,
    pub summary_en: String,
    pub summary_id: String,
    pub purpose: String,
    pub pros_en: String,
    pub pros_id: String,
    pub cons_en: String,
    pub cons_id: String,
    pub custom: bool,
}
```

### Identity rules

- Bundled providers use stable, human-readable IDs.
- Custom providers use timestamp-derived IDs.
- UI callbacks pass IDs rather than model indexes.
- Bundled records use `custom: false`.
- User-created records use `custom: true`.

## 5.2 Persistent settings

```rust
pub struct Settings {
    pub language: String,
    pub theme: String,
    pub custom_providers: Vec<DnsProvider>,
}
```

Example:

```json
{
  "language": "en",
  "theme": "system",
  "custom_providers": []
}
```

### Defaults

- Invalid or unknown language values become `en`.
- Invalid or unknown theme values become `system`.
- Missing purpose values become an empty string during deserialization and are presented as `general` while editing.
- Missing custom-provider collections become empty.
- Invalid or unreadable settings files fall back to defaults rather than preventing startup.

---

## 6. Threading and Responsiveness

The following operations run outside the Slint event-loop thread:

- Adapter discovery.
- DNS apply.
- Automatic DNS restore.
- Resolver ping checks.
- Network counter sampling.

Results are returned with `slint::invoke_from_event_loop`. UI component handles are kept weak in callbacks to avoid reference cycles.

DNS apply and reset operations set a global busy state, which displays a modal loading overlay and disables relevant actions. Network throughput sampling does not show the global overlay because it is a continuous background activity.

---

## 7. Error Handling

### Implemented error cases

- No active network adapter found.
- Selected adapter is unavailable.
- Invalid IPv4 address.
- Invalid IPv6 address.
- Address entered in the wrong IP-version field.
- Secondary address supplied without a primary address.
- No primary custom DNS address supplied.
- Selected provider has no address for the selected mode.
- PowerShell cannot be started.
- Windows rejects the DNS operation.
- Resolver does not respond to ping.
- Settings cannot be serialized or written.
- Network statistics return an invalid result.
- DNS changes attempted on an unsupported platform.

Errors and operation results are displayed through the application notification banner.

### Current limitation

Backend-originated errors are primarily English. Full error-code localization remains roadmap work.

---

## 8. Privacy and Security Considerations

- No DNS query interception is implemented.
- No analytics or telemetry dependency is included.
- No account or cloud synchronization exists.
- Custom profile data is written locally as JSON.
- PowerShell arguments escape single quotes in adapter names and DNS values.
- Addresses are parsed by Rust before custom profiles are accepted.
- DNS operations do not run on the UI thread.
- Applying DNS changes system networking and therefore intentionally depends on Windows Administrator authorization.

### Security caveats

- Ping reachability does not verify DNS response correctness or authenticity.
- Plain DNS configured through Windows is not equivalent to encrypted DNS.
- Provider metadata is bundled and is not fetched or independently verified at runtime.
- Custom DNS profiles are trusted user input after syntactic IP validation.
- The settings file is not encrypted because it contains configuration, not credentials; users should not store secrets in descriptions.

---

## 9. Platform Compatibility

### Windows

Implemented:

- Theme preference detection.
- Active adapter discovery.
- DNS address discovery.
- DNS apply.
- DNS restore.
- Adapter traffic counters.
- Resolver ping.
- Embedded executable icon.

### Non-Windows

- Slint UI may compile on supported platforms.
- A placeholder adapter can be returned.
- DNS apply and restore report that the operation is unsupported.
- Real-time Windows adapter statistics report that they are unsupported.

The current production target for the MVP is Windows.

---

## 10. Build and Validation

## 10.1 Development run

```powershell
cargo run
```

For apply and reset operations, run the terminal as Administrator.

## 10.2 Release build

```powershell
cargo build --release
```

Output:

```text
target/release/usedns.exe
```

## 10.3 Formatting

```powershell
cargo fmt --all -- --check
```

## 10.4 Tests

```powershell
cargo test
```

### Current automated test coverage

- Every bundled provider passes address validation.
- An IPv6 address in an IPv4 field is rejected.

### Validation recorded during implementation

- Slint modules compile successfully.
- Rust compilation succeeds.
- Two unit tests pass.
- No unit-test failure remains in the implemented baseline.

---

## 11. Known MVP Limitations

1. **No automatic UAC elevation**  
   The app must be launched as Administrator for DNS writes.

2. **No confirmation dialog before apply/reset**  
   Actions currently execute immediately after selection.

3. **No exact previous-configuration rollback**  
   Recovery resets to DHCP rather than restoring a previously captured static configuration.

4. **Active provider detection is address-string based**  
   The active badge compares known addresses to the Windows DNS string. Unknown or partially matching configurations remain labeled as system/custom state.

5. **Adapter changes require status refresh for all dashboard values**  
   Selecting another adapter does not guarantee every status value is immediately recomputed until Refresh runs.

6. **Network activity is not a bandwidth speed test**  
   It shows live traffic and cannot determine the connection's maximum capacity while idle.

7. **Provider catalog is bundled**  
   Provider addresses and descriptions are updated through application releases.

8. **Category mapping is currently code-defined**  
   Bundled filter membership is explicitly mapped in Rust. Custom-purpose filtering is not yet fully integrated into all provider filters.

9. **No automatic theme-change event listener**  
   Follow System is evaluated at startup or when selected, not continuously through native Windows change notifications.

10. **No system tray**  
    The Settings page explicitly labels this as unavailable.

11. **Notification banner has no timed auto-dismiss**  
    It is replaced or cleared by later application actions.

12. **No encrypted DNS protocols**  
    DNS-over-HTTPS and DNS-over-TLS are outside the MVP.

13. **Limited automated tests**  
    Platform commands, persistence, filtering, and callback orchestration need broader automated coverage.

14. **No installer or code signing**  
    The project currently produces a Cargo-built executable only.

15. **No accessibility audit**  
    Keyboard navigation, screen-reader labels, reduced motion, and high-contrast behavior require dedicated verification.

---

## 12. Roadmap

## Phase 1 — MVP Hardening

**Priority: Critical before a public stable release**

- Add a confirmation modal before applying or resetting DNS.
- Capture whether the adapter used DHCP or static DNS before a change.
- Add Restore Previous Configuration in addition to Reset to DHCP.
- Verify effective DNS addresses after PowerShell reports success.
- Add native, actionable handling for Administrator-required errors.
- Add a Windows UAC elevation flow that triggers only for privileged actions.
- Recompute DNS and connection status immediately when the adapter changes.
- Add timed notification dismissal and manual close controls.
- Localize structured backend errors into English and Indonesian.
- Add validation tests for IPv4, IPv6, optional secondary fields, and empty profiles.
- Add persistence round-trip and malformed-file tests.

## Phase 2 — Provider and Benchmark Improvements

**Priority: High**

- Move provider metadata to a versioned data file.
- Add more curated providers such as OpenDNS, Control D, and configurable NextDNS profiles.
- Add provider-policy URLs.
- Integrate custom-purpose categories with provider filters.
- Add copy-address actions.
- Add sort options for name, latency, privacy focus, and security focus.
- Implement DNS-resolution benchmarking using actual DNS queries instead of ping only.
- Show median, minimum, maximum, packet loss, and jitter.
- Clearly separate live adapter activity from controlled bandwidth testing.
- Add an optional third-party bandwidth test only after explicit consent, including data-usage disclosure.

## Phase 3 — Desktop Integration

**Priority: Medium**

- Add a Windows system tray icon.
- Add Start Minimized behavior after tray support exists.
- Add startup launch configuration.
- Add configurable connection-check and network-activity intervals.
- Add selectable speed units such as Mbps, MB/s, Kbps, and KB/s.
- Add native Windows notifications with a preference toggle.
- Observe Windows theme-change events continuously.
- Add minimize-to-tray and quick DNS switching from the tray menu.
- Preserve window size and position.

## Phase 4 — Distribution and Security

**Priority: Medium**

- Create a signed Windows installer.
- Add application version metadata to Windows resources.
- Add CI for format, test, and release builds.
- Publish checksums for release artifacts.
- Add code signing and release provenance.
- Add dependency auditing and license checks.
- Add crash logging that remains local unless users explicitly choose to export it.
- Document PowerShell execution and administrator behavior for security review.

## Phase 5 — Cross-platform Support

**Priority: Long-term**

- Introduce a formal `DnsManager` platform trait.
- Implement Linux support for NetworkManager and `systemd-resolved`.
- Implement macOS support through `networksetup` or native SystemConfiguration APIs.
- Add platform-specific adapter enumeration.
- Add platform-specific privilege handling.
- Keep provider, validation, persistence, and Slint UI logic platform-neutral.

## Phase 6 — Accessibility and UX Quality

**Priority: Ongoing**

- Complete keyboard-only navigation.
- Add explicit accessibility labels and roles.
- Verify screen-reader output.
- Verify contrast in light, dark, and Windows high-contrast modes.
- Add focus indicators for custom interactive components.
- Add responsive breakpoints or alternative layouts for small windows.
- Add empty states for searches and missing adapters.
- Add confirmation for custom-provider deletion.
- Add import/export for custom DNS profiles.
- Add onboarding explaining DNS benefits and limitations.

---

## 13. MVP Acceptance Checklist

### Core networking

- [x] Active Windows adapters can be discovered.
- [x] Current DNS addresses can be displayed.
- [x] Bundled DNS can be applied.
- [x] Automatic DNS can be restored.
- [x] IPv4-only mode is available.
- [x] IPv6-only mode is available.
- [x] Combined IPv4 and IPv6 mode is available.
- [x] Privileged-operation errors are displayed.
- [ ] UAC elevation is handled automatically.
- [ ] Previous static DNS configuration can be restored exactly.

### Connection information

- [x] Resolver reachability can be checked.
- [x] Approximate latency can be displayed.
- [x] Connection quality is classified.
- [x] Live download activity is displayed.
- [x] Live upload activity is displayed.
- [x] Traffic sampling runs outside the UI thread.
- [ ] Controlled DNS-query benchmark is available.
- [ ] Optional bandwidth speed test is available.

### Provider catalog

- [x] Bundled provider cards are available.
- [x] Provider descriptions are localized.
- [x] Advantages and limitations are shown.
- [x] Search is functional.
- [x] Category filters are functional.
- [x] Details can be expanded.
- [x] Bundled entries are immutable.
- [ ] Provider data can be updated independently of an app release.

### Custom DNS

- [x] Custom profiles can be created.
- [x] Custom profiles can be edited.
- [x] Custom profiles can be deleted.
- [x] IPv4 and IPv6 syntax is validated.
- [x] Secondary-address dependencies are validated.
- [x] Custom profiles are persisted locally.
- [x] Purpose/category is persisted.
- [x] Resolver reachability can be tested before saving.
- [ ] Deletion requires confirmation.
- [ ] Profiles can be imported or exported.

### Localization and appearance

- [x] English is available and is the default.
- [x] Indonesian is available.
- [x] Language preference is persisted.
- [x] Light mode is available.
- [x] Dark mode is available.
- [x] Follow System mode is available.
- [x] Theme preference is persisted.
- [x] Major UI surfaces use shared theme tokens.
- [x] UI icons adhere strictly to the Lucide icon library via `lucide-slint`.
- [ ] Native Windows theme changes are observed continuously.
- [ ] Full accessibility audit is complete.

### Policy and privacy

- [x] Privacy policy content is available in-app.
- [x] Third-party provider responsibility is explained.
- [x] Administrator requirements are explained.
- [x] Ping versus bandwidth testing is explained.
- [x] Local settings storage is explained.
- [x] The application does not include telemetry.

### Build and quality

- [x] Slint UI is split into maintainable modules.
- [x] Application icon is used in the window.
- [x] Multi-resolution Windows executable icon is generated.
- [x] Formatting checks pass.
- [x] Existing unit tests pass.
- [ ] Platform operations have automated integration tests.
- [ ] Signed installer and release automation are available.

---

## 14. Suggested Definition of Done for Version 1.0

UseDNS can be considered ready for a `1.0` Windows release when:

1. DNS apply, verification, reset, and previous-state rollback are reliable across common Ethernet, Wi-Fi, VPN, and virtual adapters.
2. Privilege elevation is explicit, safe, and understandable.
3. All user-facing errors are localized.
4. Provider metadata has documented sources and update procedures.
5. Persistence and platform operations have meaningful automated coverage.
6. Keyboard navigation and screen-reader behavior are verified.
7. Release executables are signed and distributed through a reproducible pipeline.
8. Privacy, disclaimer, and third-party provider information have been reviewed.
9. Installer, upgrade, and uninstall flows are tested.
10. No UI option suggests behavior that is not actually implemented.

---

## 15. Reference Commands

Run during development:

```powershell
cargo run
```

Run as Administrator when testing DNS changes:

```powershell
cargo run
```

Validate formatting and tests:

```powershell
cargo fmt --all -- --check
cargo test
```

Create a release executable:

```powershell
cargo build --release
```

The expected release artifact is:

```text
target/release/usedns.exe
```
