# Architecture

This document describes the structure and data flow of the REEK Ultimate
Uninstaller workspace. It reflects the **current** source layout (not the
original design spec).

## Workspace layout

```
reek/
├── Cargo.toml              # workspace manifest, shared deps, release profiles
├── audit.toml              # cargo-audit config
├── deny.toml               # cargo-deny config (licenses, bans, sources, advisories)
├── clippy.toml             # clippy thresholds
├── rustfmt.toml            # rustfmt options
├── Makefile                # local dev commands
├── crates/
│   ├── greek-common/       # shared types, errors, constants, traits
│   ├── greek-core/         # business logic: scanners, uninstaller, leftovers, config
│   ├── greek-windows/      # Windows-only capabilities (registry, services, restore, WMI)
│   ├── greek-platform/     # Linux/macOS scanners + common platform helpers
│   ├── greek-cli/          # headless CLI binary `reek`
│   └── greek-tui/          # ratatui terminal app binary `reek-tui`
└── docs/                   # this documentation set
```

## Crate responsibilities

### greek-common (foundation, no platform code)

- `models.rs` — `InstalledApp`, `UninstallOptions`, `UninstallResult`,
  `LeftoverArtifact`, `SafetyLevel`, `ArtifactType`, `RegistryKey`,
  `RegistryHive`, `BatchQueue`, `GreekConfig`, publisher-name cleaning, and the
  `SystemStats` family (cross-platform surface).
- `error.rs` — `GreekError` and domain error enums.
- `constants.rs` — timeouts, size limits, `PROTECTED_PATHS`.
- `traits.rs` — `AppScanner`, `UninstallStrategy`, `RestorePointManager`
  (trait), `SystemStatsCollector` (trait).
- `lib.rs` — re-exports.

### greek-core (decides)

- `scanner.rs` — `ScannerManager` aggregating platform scanners; rayon
  parallel scan helpers (Mutex-free, per-fork result merging).
- `uninstaller.rs` — `UninstallerManager` + strategies: `Standard` (runs
  uninstall string), `Msi` (extracts product code via UUID regex), `ForceRemove`
  (kills processes, deletes files/registry). Contains the quote-aware
  `parse_command_string` and timeout-bound execution.
- `app_service.rs` — `GreekAppService`: orchestration layer, restore-point
  creation, force-remove, leftover analysis, batch execution, event broadcast.
- `leftover.rs` — leftover analyzers and `LeftoverAnalyzerManager`.
- `browser_extensions.rs`, `task_scheduler.rs`, `windows_features.rs` — scanners.
- `config.rs` — `ConfigManager` (TOML in platform config dir, validated).
- `utils.rs` — hashing, size, path safety helpers, `delete_registry_key`.

### greek-windows (executes, Windows-only by nature)

- `registry.rs` — registry scanner using `winreg` + `windows` crate.
- `restore.rs` — restore-point manager (PowerShell `Checkpoint-Computer`).
- `services.rs` — service enumeration / stop / delete.
- `wmi.rs` — WMI queries (installed software, features, processes).
- `store.rs` — Windows Store (AppX) package scanning.
- `system_stats.rs` — live system stats; re-exports the cross-platform
  `greek_common` types.
- `icon.rs` — icon extraction + dominant-color.
- `lib.rs` — module wiring with `#[cfg(not(target_os = "windows"))]` stubs so the
  crate compiles everywhere (empty/no-op implementations), since
  `greek-core`/`greek-tui` can build with the `windows` feature on other OSes.

Non-Windows compilation is handled via feature + OS gating in the dependants:

```rust
#[cfg(all(target_os = "windows", feature = "windows"))]
```

### greek-platform (scans Linux/macOS)

- `common.rs` — `get_os`, `get_arch`, `is_elevated`, `get_common_app_dirs`.
- `linux.rs` — `LinuxPackageScanner` (`apt`/`rpm`/`flatpak` via CLI commands).
- `macos.rs` — `MacOsAppScanner` (.app bundles, Info.plist via `plutil`).
- Lib.rs compiles all modules on every platform; platform deps are
  target-gated in the manifest.

### greek-cli (`reek`)

- clap-based subcommands: `scan`, `uninstall`, `force-remove`, `leftovers`,
  `restore-point`, `completions`, `config`. Uses `color-eyre` reporting.

### greek-tui (`reek-tui`)

- ratatui + crossterm event loop with a fuzzy-search list, detail panel,
  context menu, and (on Windows with the `windows` feature) a live stats bar
  fed by a background `SystemStatsCollector` thread over an mpsc channel.

## Data flow

### Scan

```
ScannerManager
  ├─ greek-windows::WindowsRegistryScanner  (windows feature)
  ├─ greek-windows::WindowsStoreScanner     (windows feature)
  ├─ greek-platform::LinuxScannerAdapter    (linux)
  ├─ greek-platform::MacOsScannerAdapter    (macos)
  └─ user-registered scanners
        └─ ▶ Vec<InstalledApp>
```

### Uninstall

```
GreekAppService::uninstall_app
  ├─ [create restore point if requested]
  └─ UninstallerManager::uninstall
       ├─ MsiUninstallStrategy      (MSI product code present)
       ├─ StandardUninstallStrategy (uninstall_string present)
       └─ ForceRemoveStrategy       (fallback / force)
            ├─ kill_processes_by_path
            ├─ delete install_location
            └─ delete_registry_key per registry entry
```

### Leftover analysis

```
GreekAppService::analyze_leftovers
  └─ LeftoverAnalyzerManager::analyze_app
       └─ per analyzer → Vec<LeftoverArtifact> (SafetyLevel + confidence)
```

## Feature gates

| Feature | crates | Enables |
|---------|--------|---------|
| `windows` | greek-core, greek-tui | pulls `greek-windows` |

Both crates declare `[features] windows = ["greek-windows"]`; usage sites use
`#[cfg(all(target_os = "windows", feature = "windows"))]` so `--all-features`
works on every OS (on non-Windows the stub modules in `greek-windows/lib.rs`
are used).

## Release profiles (workspace Cargo.toml)

```toml
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

## Related docs

- [CI/CD](CI_CD.md) — pipeline jobs, gates, local equivalents.
- [Security](SECURITY.md) — layered controls.
- [Security policy](../SECURITY.md) — threats, disclosure.