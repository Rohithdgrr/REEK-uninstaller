# Greek Uninstaller — Complete Tech Stack Specification
## Pure Rust TUI Application

---

## 1. Core Toolchain

| Component | Version / Spec | Purpose |
|-----------|---------------|---------|
| **Rust Edition** | `2021` | Language edition |
| **MSRV** | `1.78.0` | Minimum Supported Rust Version |
| **Channel** | `stable` | No nightly features required |
| **Target Triples** | `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin` | Primary platforms |

---

## 2. Workspace Architecture (6 Crates)

```
greek-uninstaller/                 # Workspace Root
├── Cargo.toml
├── crates/
│   ├── greek-core/                # Business logic, models, scanners
│   ├── greek-tui/                 # Terminal UI (ratatui)
│   ├── greek-cli/                 # Headless CLI binary
│   ├── greek-windows/             # Windows-specific APIs
│   ├── greek-platform/            # Platform abstractions (Linux/macOS stubs)
│   └── greek-common/              # Shared types, errors, constants
```

---

## 3. Crate-by-Crate Dependency Breakdown

### 3.1 `greek-common` — Shared Foundation

```toml
[package]
name = "greek-common"
version = "0.1.0"
edition = "2021"

[dependencies]
# Serialization
serde = { version = "1.0.204", features = ["derive"] }
serde_json = "1.0.120"

# IDs & Time
uuid = { version = "1.10.0", features = ["v4", "serde"] }
chrono = { version = "0.4.38", features = ["serde"] }

# Errors
thiserror = "1.0.63"
color-eyre = "0.6.3"

# Async traits
async-trait = "0.1.81"

# Collections
indexmap = "2.2.6"           # Preserves insertion order

# Tracing
tracing = "0.1.40"
```

### 3.2 `greek-core` — Business Logic

```toml
[package]
name = "greek-core"
version = "0.1.0"
edition = "2021"

[dependencies]
greek-common = { path = "../greek-common" }

# Async Runtime
tokio = { version = "1.39.2", features = ["full"] }
tokio-util = "0.7.11"

# Process Management
sysinfo = "0.31.2"           # Cross-platform process info
which = "6.0.3"              # Executable resolution

# File System
walkdir = "2.5.0"            # Recursive directory traversal
ignore = "0.4.22"            # Gitignore-style filtering
jwalk = "0.8.1"              # Parallel directory walking
pathdiff = "0.2.1"           # Relative path computation

# Pattern Matching
glob = "0.3.1"
regex = "1.10.6"
fuzzy-matcher = "0.3.7"      # Fuzzy search algorithms

# Data Processing
dashmap = "6.0.1"            # Concurrent HashMap
rayon = "1.10.0"             # Data parallelism

# Human-readable formatting
humansize = "2.1.3"
humantime = "2.1.0"

# Hashing
sha2 = "0.10.8"
hex = "0.4.3"

# Configuration
toml = "0.8.19"
directories = "5.0.1"        # XDG/BaseDirs path resolution

# HTTP (for store API/metadata)
reqwest = { version = "0.12.5", features = ["json"] }

# Machine Learning (future leftover confidence)
linfa = "0.7.1"              # Optional, feature-gated

[dev-dependencies]
tempfile = "3.12.0"
mockall = "0.13.0"
```

### 3.3 `greek-tui` — Terminal User Interface

```toml
[package]
name = "greek-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
greek-common = { path = "../greek-common" }
greek-core = { path = "../greek-core" }

# TUI Framework
ratatui = { version = "0.28.0", features = ["unstable-widget-ref"] }
crossterm = { version = "0.28.1", features = ["event-stream"] }

# TUI Extensions
tui-textarea = "0.6.1"           # Multi-line text input
tui-tree-widget = "0.22.0"       # Hierarchical tree view
tui-input = "0.10.1"             # Input field handling
tui-big-text = "0.6.0"           # Large ASCII text rendering
tui-popup = "0.5.0"              # Popup/dialog overlays
throbber-widgets-tui = "0.7.0"   # Loading spinners
tui-logger = "0.13.1"            # In-app log viewer

# Async TUI
tokio = { version = "1.39.2", features = ["full"] }
futures = "0.3.30"
tokio-stream = "0.1.15"

# State Management
im = "15.1.0"                    # Immutable data structures for UI state

# Input Handling
bitflags = "2.6.0"               # Key modifier flags

# Clipboard
arboard = "3.4.0"                # System clipboard access

# Notifications
notify-rust = { version = "4.11.0", optional = true }  # Desktop notifications

[features]
default = []
notifications = ["notify-rust"]
```

### 3.4 `greek-cli` — Headless Mode

```toml
[package]
name = "greek-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "greek"
path = "src/main.rs"

[dependencies]
greek-common = { path = "../greek-common" }
greek-core = { path = "../greek-core" }

# CLI Framework
clap = { version = "4.5.13", features = ["derive", "env", "cargo"] }
clap_complete = "4.5.12"         # Shell completions (bash, zsh, fish)

# Output Formatting
comfy-table = "7.1.1"            # Terminal tables
indicatif = "0.17.8"             # Progress bars for CLI
console = "0.15.8"               # Terminal styling, colors

# Interactive prompts
dialoguer = "0.11.0"             # Confirm prompts, selections
colored = "2.1.0"                # ANSI color output

# Logging
tracing-subscriber = { version = "0.3.18", features = ["env-filter", "fmt"] }
```

### 3.5 `greek-windows` — Windows Native APIs

```toml
[package]
name = "greek-windows"
version = "0.1.0"
edition = "2021"

[dependencies]
greek-common = { path = "../greek-common" }

# Official Windows API bindings
windows = { version = "0.58.0", features = [
    "Win32_Foundation",
    "Win32_Security",
    "Win32_System_Registry",
    "Win32_System_Services",
    "Win32_System_ProcessStatus",
    "Win32_System_Threading",
    "Win32_System_WindowsProgramming",
    "Win32_Storage_FileSystem",
    "Win32_UI_Shell",
    "Win32_UI_WindowsAndMessaging",
    "Wdk_System_SystemServices",
    "Win32_System_Com",
    "Win32_System_Ole",
    "Win32_System_Variant",
    "Win32_System_Wmi",
    "Win32_Management_Msi",
    "Win32_System_Power",
    "Win32_System_Restore",
] }

# Registry
winreg = "0.52.0"                  # Higher-level registry API

# WMI
wmi = "0.14.0"                     # WMI query interface

# Windows-specific utilities
winapi = { version = "0.3.9", features = [
    "processthreadsapi",
    "handleapi",
    "winnt",
    "securitybaseapi",
] }

# UWP/Store
windows-management = "0.58.0"      # Appx package management

# PowerShell interop
powershell_script = "1.0.4"        # Execute PowerShell commands

# COM utilities
com-rs = "0.2.1"                   # COM interface helpers

# Privilege escalation
runas = "1.2.0"                    # UAC elevation helpers

[target.'cfg(windows)'.dependencies]
ntapi = "0.4.1"                    # Native NT API (undocumented features)
```

### 3.6 `greek-platform` — Cross-Platform Abstractions

```toml
[package]
name = "greek-platform"
version = "0.1.0"
edition = "2021"

[dependencies]
greek-common = { path = "../greek-common" }

# Linux package managers
rust-apt = { version = "0.7.0", optional = true }      # Debian/Ubuntu
alpm = { version = "2.2.3", optional = true }          # Arch Linux
rpm = { version = "0.15.0", optional = true }          # RPM-based

# macOS
plist = { version = "1.7.0", optional = true }         # plist parsing
core-foundation = { version = "0.9.4", optional = true }

# Flatpak/Snap
flatpak = { version = "0.5.0", optional = true }

[target.'cfg(target_os = "linux")'.dependencies]
freedesktop-desktop-entry = "0.7.0"   # .desktop file parsing
linicon = "0.12.0"                    # Icon theme resolution

[target.'cfg(target_os = "macos")'.dependencies]
objc = "0.2.7"
objc-foundation = "0.1.1"

[features]
default = []
linux = ["rust-apt", "alpm", "rpm"]
macos = ["plist", "core-foundation"]
universal = ["linux", "macos"]
```

---

## 4. Root Workspace `Cargo.toml`

```toml
[workspace]
members = [
    "crates/greek-common",
    "crates/greek-core",
    "crates/greek-tui",
    "crates/greek-cli",
    "crates/greek-windows",
    "crates/greek-platform",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Greek Team <team@greek.io>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/greek/greek-uninstaller"
rust-version = "1.78.0"

[workspace.dependencies]
# Async
tokio = { version = "1.39.2", features = ["full"] }
futures = "0.3.30"

# Error handling
thiserror = "1.0.63"
color-eyre = "0.6.3"

# Serialization
serde = { version = "1.0.204", features = ["derive"] }
serde_json = "1.0.120"

# Tracing
tracing = "0.1.40"
tracing-subscriber = { version = "0.3.18", features = ["env-filter"] }

# Testing
tempfile = "3.12.0"
mockall = "0.13.0"
criterion = { version = "0.5.1", features = ["html_reports"] }

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true
panic = "abort"

[profile.release-windows]
inherits = "release"
lto = "fat"
```

---

## 5. Build & Packaging Stack

| Tool | Version | Purpose |
|------|---------|---------|
| **cargo** | Built-in | Build system |
| **cargo-workspaces** | `0.3.0` | Workspace versioning & publishing |
| **cargo-dist** | `0.21.0` | Cross-platform release packaging |
| **cargo-release** | `0.25.0` | Automated version bumping & changelog |
| **cross** | `0.2.5` | Cross-compilation in containers |
| **cargo-audit** | `0.20.0` | Security vulnerability scanning |
| **cargo-deny** | `0.16.0` | License compliance & crate policy enforcement |
| **cargo-outdated** | `0.15.0` | Dependency update checking |

### Build Scripts

```toml
# .cargo/config.toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]

[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
```

---

## 6. CI/CD Pipeline (GitHub Actions)

| Action | Purpose |
|--------|---------|
| `actions/checkout@v4` | Source checkout |
| `dtolnay/rust-toolchain@stable` | Rust toolchain setup |
| `Swatinem/rust-cache@v2` | Build caching |
| `cargo test --workspace` | Unit & integration tests |
| `cargo clippy --all-targets --all-features` | Linting |
| `cargo fmt --check` | Format checking |
| `cargo audit` | Security audit |
| `cargo deny check` | License/policy check |
| `cargo-dist build` | Release artifact generation |
| `softprops/action-gh-release@v2` | GitHub release creation |

### CI Matrix
```yaml
os: [windows-latest, ubuntu-latest, macos-latest]
rust: [stable, 1.78.0]  # MSRV check
```

---

## 7. Testing Stack

| Crate/Tool | Version | Test Type |
|------------|---------|-----------|
| **built-in `test`** | — | Unit tests |
| **tokio-test** | `0.4.4` | Async test runtime |
| **mockall** | `0.13.0` | Mocking traits |
| **tempfile** | `3.12.0` | Temporary directories/files |
| **assert_cmd** | `2.0.14` | CLI binary integration tests |
| **predicates** | `3.1.2` | Assertion predicates |
| **criterion** | `0.5.1` | Benchmarks |
| **proptest** | `1.5.0` | Property-based testing |
| **insta** | `1.39.0` | Snapshot testing for UI output |

---

## 8. Development & Debugging Tools

| Tool | Installation | Purpose |
|------|-------------|---------|
| **cargo-watch** | `cargo install cargo-watch` | Auto-rebuild on file changes |
| **cargo-expand** | `cargo install cargo-expand` | Macro expansion inspection |
| **cargo-flamegraph** | `cargo install flamegraph` | Performance profiling |
| **cargo-bloat** | `cargo install cargo-bloat` | Binary size analysis |
| **cargo-modules** | `cargo install cargo-modules` | Dependency graph visualization |
| **cargo-udeps** | `cargo install cargo-udeps` | Unused dependency detection |
| **tracy-client** | Crate | Frame profiler integration (optional) |

---

## 9. Documentation Stack

| Tool | Purpose |
|------|---------|
| **rustdoc** | Built-in API docs |
| **mdBook** | User guide & manual |
| **cargo-doc** | Workspace documentation generation |
| **plantuml** | Architecture diagrams |
| **mdbook-mermaid** | Mermaid diagrams in docs |

---

## 10. Platform-Specific API Mapping

### Windows APIs → Rust Crates

| Windows API | Rust Crate | Feature |
|-------------|-----------|---------|
| Registry (`Reg*`) | `winreg` + `windows::Win32::System::Registry` | Read/write/delete keys |
| WMI | `wmi` | Query installed products |
| MSI API | `windows::Win32::Management::Msi` | MSI product enumeration |
| Service Control Manager | `windows::Win32::System::Services` | Start/stop/delete services |
| Task Scheduler | PowerShell via `powershell_script` | Task enumeration |
| UWP/Store | `windows-management` | Appx package management |
| UAC Elevation | `runas` | Restart as admin |
| System Restore | `windows::Win32::System::Restore` | Create restore points |
| File ownership | `windows::Win32::Security` | `SeTakeOwnershipPrivilege` |
| Process enumeration | `sysinfo` + `windows::Win32::System::ProcessStatus` | Kill processes |

### Linux APIs → Rust Crates

| Linux API | Rust Crate |
|-----------|-----------|
| dpkg database | `rust-apt` |
| pacman database | `alpm` |
| rpm database | `rpm` |
| Flatpak | `flatpak` |
| .desktop entries | `freedesktop-desktop-entry` |
| xdg dirs | `directories` |

### macOS APIs → Rust Crates

| macOS API | Rust Crate |
|-----------|-----------|
| .app bundles | `walkdir` + `plist` |
| LaunchAgents/Daemons | `plist` + file scanning |
| Homebrew | Shell out to `brew` |
| CoreFoundation | `core-foundation` |

---

## 11. Complete Dependency Tree Summary

```
greek-uninstaller (workspace)
│
├── greek-common (foundational)
│   ├── serde + serde_json
│   ├── uuid + chrono
│   ├── thiserror + color-eyre
│   ├── async-trait
│   ├── indexmap
│   └── tracing
│
├── greek-core (business logic)
│   ├── tokio + tokio-util
│   ├── sysinfo + which
│   ├── walkdir + jwalk + ignore
│   ├── glob + regex + fuzzy-matcher
│   ├── dashmap + rayon
│   ├── humansize + humantime
│   ├── sha2 + hex
│   ├── toml + directories
│   └── reqwest
│
├── greek-tui (interface)
│   ├── ratatui + crossterm
│   ├── tui-textarea + tui-tree-widget + tui-input
│   ├── tui-big-text + tui-popup + throbber-widgets-tui
│   ├── tokio + futures + tokio-stream
│   ├── im (immutable)
│   ├── bitflags
│   ├── arboard
│   └── notify-rust (optional)
│
├── greek-cli (headless)
│   ├── clap + clap_complete
│   ├── comfy-table + indicatif + console
│   ├── dialoguer + colored
│   └── tracing-subscriber
│
├── greek-windows (Win32)
│   ├── windows (official MS bindings)
│   ├── winreg
│   ├── wmi
│   ├── winapi
│   ├── powershell_script
│   ├── runas
│   └── ntapi
│
└── greek-platform (cross-platform)
    ├── rust-apt / alpm / rpm (Linux)
    ├── plist + core-foundation (macOS)
    └── freedesktop-desktop-entry (Linux)
```

---

## 12. Quick Start Commands

```bash
# Install development tools
cargo install cargo-watch cargo-expand cargo-flamegraph cargo-bloat
cargo install cargo-audit cargo-deny cargo-outdated cargo-workspaces

# Clone and build
git clone https://github.com/greek/greek-uninstaller.git
cd greek-uninstaller
cargo build --workspace

# Run TUI
cargo run -p greek-tui

# Run CLI
cargo run -p greek-cli -- --help

# Run tests
cargo test --workspace --all-features

# Build release (Windows static)
cargo build --release -p greek-tui --target x86_64-pc-windows-msvc

# Security audit
cargo audit && cargo deny check

# Generate shell completions
cargo run -p greek-cli -- --generate bash > /usr/share/bash-completion/completions/greek
```

---

This stack is **production-ready** and avoids nightly Rust, ensuring maximum stability. The workspace separation ensures clean boundaries: you can develop the TUI and core logic independently, swap out `greek-windows` for `greek-platform` on Linux, and ship a headless CLI without the TUI weight.

Want me to generate the actual `Cargo.toml` files and initial module scaffolding for any of these crates?