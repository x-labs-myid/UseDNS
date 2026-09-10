<p align="center">
  <img src=".assets/UseDNS.png" alt="UseDNS logo" width="180">
</p>

<h1 align="center">UseDNS</h1>

<p align="center"><strong>Choose. Switch. Connect.</strong></p>

UseDNS is a lightweight desktop utility for discovering, comparing, and applying DNS resolvers without manually editing Windows network settings. It is built with [Rust](https://www.rust-lang.org/) and [Slint](https://slint.dev/).

> [!NOTE]
> UseDNS is currently an MVP. DNS changes are supported on Windows; additional platform backends are planned.

## Highlights

- Detect active network adapters and their current DNS configuration
- Compare curated public DNS providers with clear benefits and trade-offs
- Apply IPv4, IPv6, or both address families
- Create, update, and remove custom DNS profiles
- Restore automatic DNS configuration from DHCP
- Monitor resolver reachability and approximate latency
- Switch between English and Indonesian, with English as the default
- Choose a glass-inspired light or dark appearance, or follow the Windows theme
- Keep preferences and custom profiles on the local device

## Included DNS Providers

| Provider          | Primary benefit                     | IPv4 | IPv6 |
| ----------------- | ----------------------------------- | :--: | :--: |
| Cloudflare        | Performance and privacy             | Yes  | Yes  |
| Google Public DNS | Reliability and global availability | Yes  | Yes  |
| Quad9             | Malware and phishing protection     | Yes  | Yes  |
| AdGuard DNS       | Ad and tracker filtering            | Yes  | Yes  |

Built-in profiles are read-only to protect their verified configuration. User-created profiles remain fully editable.

## Requirements

- Windows 10 or Windows 11
- PowerShell with the `DnsClient` module
- Administrator privileges when applying or resetting DNS
- Rust 1.92 or newer when building from source

Reading network status does not require elevation. Windows only requires Administrator privileges when UseDNS writes DNS settings.

## Getting Started

Clone the repository and enter its directory:

```sh
git clone https://github.com/x-labs-myid/UseDNS.git
cd UseDNS
```

Run the development build:

```sh
cargo run
```

To apply or reset DNS, launch the terminal as **Administrator** before running the command.

Create an optimized executable:

```sh
cargo build --release
```

The resulting executable is written to `target/release/usedns.exe`.

## Usage

1. Open **Home** and select an active network adapter.
2. Review its effective DNS addresses and connection status.
3. Open **DNS providers** and select IPv4, IPv6, or the combined mode.
4. Review a provider's purpose, advantages, and limitations.
5. Select **Use DNS** to apply it.
6. If connectivity is affected, return to **Home** and select **Use automatic DNS**.

Custom resolvers such as Pi-hole, AdGuard Home, private DNS servers, and internal network resolvers can be managed from **Custom DNS**.

## Architecture

```text
UseDNS/
├── .assets/             # Application artwork
├── src/
│   ├── main.rs          # Application state and UI callbacks
│   ├── models.rs        # DNS provider model and validation
│   ├── storage.rs       # Local settings persistence
│   └── system.rs        # Windows adapter and DNS operations
├── ui/
│   ├── components/
│   │   ├── common.slint  # Shared cards, titles, tags, and theme controls
│   │   └── sidebar.slint # Application navigation
│   ├── pages/
│   │   ├── home.slint
│   │   ├── providers.slint
│   │   ├── custom-dns.slint
│   │   ├── policy.slint
│   │   └── settings.slint
│   ├── app-window.slint  # Composition root and Rust callback surface
│   ├── theme.slint       # Shared light/dark design tokens
│   └── types.slint       # UI-facing data structures
├── build.rs             # UI compilation and Windows resources
└── Cargo.toml
```

UseDNS calls the Windows `Set-DnsClientServerAddress` PowerShell command to apply and reset DNS settings. Platform-specific operations are kept separate from the provider model and UI.

## Data and Privacy

UseDNS does not operate a DNS resolver and does not inspect, record, or sell DNS queries. Queries are handled directly by the provider selected by the user. Each provider has independent logging, filtering, privacy, and retention policies.

Custom DNS profiles and language preferences are serialized to the operating system's local application configuration directory. UseDNS does not upload this data.

The connection indicator checks resolver reachability and approximate ping latency. It is not a bandwidth test and does not send test files to a third-party speed-testing service.

## Documentation

The complete MVP product specification, implementation inventory, known limitations, acceptance checklist, and roadmap are available in [`docs/spec/MVP_SPECIFICATION.md`](docs/spec/MVP_SPECIFICATION.md).

## Development

Format the project and run its tests before submitting a change:

```sh
cargo fmt --all -- --check
cargo test
```

The current tests cover validation of bundled DNS profiles and rejection of addresses with an incorrect IP version.

## Roadmap

Potential follow-up work includes:

- Native elevation flow for DNS operations
- Linux and macOS DNS backends
- DNS response benchmarking and recommendation ranking
- Confirmation and rollback of the previous adapter configuration
- Search and filtering for larger provider catalogs
- Signed Windows installers and release automation

## Disclaimer

UseDNS is an independent utility and is not affiliated with, endorsed by, or sponsored by any listed DNS provider. Provider names and trademarks belong to their respective owners.

Changing DNS does not make a connection anonymous or completely private. Availability and performance vary by ISP, location, selected provider, and network conditions.

## License

UseDNS is available under the [MIT License](LICENSE).
