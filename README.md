# REEK Ultimate Uninstaller

The uninstaller that actually uninstalls. A cross-platform application
uninstaller written in pure Rust, with a terminal UI (`reek-tui`) and a
headless CLI (`reek`). REEK scans for installed applications, uninstalls them
(standard, MSI, or force-remove), analyzes leftovers, and on Windows can create
system restore points before uninstalling.

> **Status:** early development (0.1.x). Expect breaking changes.

## Features

- **Application scanning** — Windows Registry + Store, Linux package managers
  (apt/rpm/flatpak via CLI), macOS `.app` bundles.
- **Three uninstall strategies** — standard uninstall string, MSI product-code
  uninstall, and force-remove (kill processes, delete files + registry keys).
- **Leftover analysis** — detects leftover files, directories, registry keys,
  services, and tasks, each tagged with a `SafetyLevel` and confidence score.
- **System Restore points** (Windows) — created before uninstall by default.
- **Live system stats** (Windows TUI) — CPU/RAM/disk/process metrics in a
  status bar.
- **Two front-ends** — a ratatui terminal UI and a scriptable CLI.
- **Security-first design** — protected-path safeguards, quote-aware command
  parsing, timeouts, pinned dependencies. See [SECURITY](SECURITY.md) and
  [docs/SECURITY](docs/SECURITY.md).

## Requirements

- Rust **1.88.0+** (MSRV). No nightly features used.
- Windows builds need the MSVC toolchain (via rustup `x86_64-pc-windows-msvc`).
- Linux: no native bindings required — scanners shell out to `apt`/`rpm`/
  `flatpak`/`brew` when present.

## Workspace

```
crates/
├── greek-common    shared types, errors, constants, traits
├── greek-core      business logic: scanners, uninstaller, leftovers, config
├── greek-windows   Windows capabilities (registry, services, restore, WMI)
├── greek-platform  Linux/macOS scanners + common platform helpers
├── greek-cli       headless CLI binary  → reek
└── greek-tui       terminal UI binary   → reek-tui
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Build & run

```bash
# Build everything
cargo build --workspace

# Run the TUI
cargo run --bin reek-tui

# Run the CLI
cargo run --bin reek -- --help
```

### CLI usage

```bash
# List installed applications
reek list

# Search
reek search <query> [--fuzzy]

# Show app details
reek info <name>

# Uninstall (confirm prompt; -y to skip, --force for force-remove)
reek uninstall <name> [-y] [--force] [--silent]

# Scan/clean leftovers
reek scan --leftovers
reek clean --leftovers -y

# Create a system restore point (Windows)
reek restore-point

# Shell completions
reek completions bash
```

### Windows-specific build (static CRT)

The workspace ships a `release-windows` profile (fat LTO). Release binaries
that don't require the VC runtime can be built with:

```bash
cargo build --release --profile release-windows -p greek-cli -p greek-tui
```

## Testing & quality gates

```bash
cargo test --workspace --all-features --locked   # unit + integration tests
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check                        # formatting
cargo audit                                      # dependency vulnerabilities
cargo deny check advisories bans licenses sources # licenses / bans / advisories
cargo doc --workspace --all-features --no-deps    # docs
```

All of the above run automatically in CI on Linux, macOS, and Windows
(see [docs/CI_CD.md](docs/CI_CD.md)). `make ci` runs the quality gates locally.

## Security

- [SECURITY.md](SECURITY.md) — threat model, supported versions, vulnerability
  reporting.
- [docs/SECURITY.md](docs/SECURITY.md) — layered security architecture.
- Security tooling: `cargo-audit`, `cargo-deny`, committed `Cargo.lock`, pinned
  CI action SHAs, least-privilege CI permissions.

## Documentation

| Doc | Contents |
|-----|----------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Crate layout, data flow, feature gates |
| [docs/CI_CD.md](docs/CI_CD.md) | CI jobs, gates, security, action pinning |
| [docs/RELEASING.md](docs/RELEASING.md) | Release process & checklist |
| [docs/SECURITY.md](docs/SECURITY.md) | Security architecture & controls |
| [INSTALL.md](INSTALL.md) | Installing the Rust toolchain & building |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution guide |
| [CHANGELOG.md](CHANGELOG.md) | Notable changes |
| [PRD.md](PRD.md), [features.md](features.md) | Product requirements & feature spec |

## License

MIT OR Apache-2.0. See [LICENSE](LICENSE).

## Support

- Security issues: see [SECURITY.md](SECURITY.md) — do **not** use public issues.
- Bugs & features: open a GitHub issue or PR per [CONTRIBUTING.md](CONTRIBUTING.md).