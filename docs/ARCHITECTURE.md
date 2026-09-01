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

### New modules (desktop app)

- `video.rs` (`crates/greek-core/src/video.rs`) — `VideoScanner`: whole-device scan for videos (34 exts `mp4, mkv, avi…`), roots `C:\Users\*\Videos|Downloads` + every drive `D:\Movies`, depth-limited `WalkDir`, `>1 MiB`, `is_protected_path` skip, dedup + size sort. Used by Tauri `scan_videos`/`delete_videos`. **Excludes** `EXCLUDED_DOC_IMAGE_EXTS` (`pdf, doc, ppt, xlsx, jpg, png…`) — never flag documents/images.
- `dev_modules.rs` (`crates/greek-core/src/dev_modules.rs`) — `DevModulesScanner`: hunts `node_modules`, `target` (Rust/Java), Python `venv/.venv/__pycache__`, `dist/build/out/.next/.nuxt|vendor/.gradle/.parcel-cache` etc. Roots `Documents/Projects/code/dev/workspace` for every user + every drive. `walk_scan` depth 6, `should_skip_dir` avoids `C:\Windows`, `.git`, calculates `size_bytes` + `file_count` via `WalkDir`. One-tap delete via `delete_modules`/`delete_all` (protected-path + known-pattern guard). Tauri `scan_dev_modules`/`clean_dev_modules`.
- `src-tauri` desktop: React `VideoVault.tsx`, `DevCleaner.tsx`, `SuccessTickDialog.tsx` (UPI green tick), header tabs `Apps|Movies|Dev Cleaner` (`src/App.tsx`). Tauri `plugin-updater` (`tauri.conf.json:plugins.updater`, `createUpdaterArtifacts:true`) for auto-update (see `docs/AUTO_UPDATE.md`).

**Safety: documents & images never deleted**
`crates/greek-core/src/leftover.rs:EXCLUDED_DOC_IMAGE_EXTS` lists `pdf, doc, docx, ppt, pptx, xls, xlsx, jpg, jpeg, png, gif, bmp, webp, svg, heic…` — every filesystem/junk/duplicate scanner skips `is_excluded_doc_image(path)` even if filename contains app token. Movies vault is explicit opt-in with its own video-only allowlist (`video.rs:VIDEO_EXTS`).

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

### Leftover analysis (whole-device)

```
GreekAppService::analyze_leftovers  (registers 6 analyzers)
  └─ LeftoverAnalyzerManager::analyze_app
       ├─ FileSystemLeftoverAnalyzer (all drives Program Files/Users/AppData/Windows, tokenized, accurate dir size)
       ├─ RegistryLeftoverAnalyzer (HKLM/HKCU Software, Run, Services, Uninstall, Classes)
       ├─ JunkLeftoverAnalyzer (Windows\Temp, %TEMP%, Prefetch, cache)
       ├─ ServiceLeftoverAnalyzer (Win32_Service)
       ├─ TaskLeftoverAnalyzer (Get-ScheduledTask)
       ├─ ShortcutLeftoverAnalyzer (Start Menu/Desktop *.lnk)
       └─ DuplicateDownloadAnalyzer (Downloads/Desktop + drive roots, installer exts, safe)
       └─ → Vec<LeftoverArtifact> (SafetyLevel + confidence, size, grouped by drive in UI)
       + EXCLUDED_DOC_IMAGE_EXTS guard — never returns .pdf/.doc/.ppt/.xlsx/.jpg/.png etc.
```

### Video vault (Movies)

```
Tauri scan_videos → VideoScanner::scan_all (build_roots: Users\Videos|Downloads + every drive Videos/Movies)
  └─ WalkDir depth 4-6, VIDEO_EXTS filter (mp4, mkv… 34), >1 MiB, is_protected_path skip
  └─ → Vec<VideoEntry {path,size,drive}> sorted by size desc → VideoVault.tsx grouped by drive
Tauri delete_videos → VideoScanner::delete_videos (recycle-safe)
```

### Dev Cleaner (one-click purge)

```
Tauri scan_dev_modules → DevModulesScanner::scan_all (roots: Documents/Projects/code + every drive)
  └─ walk_scan depth 6, PATTERNS [node_modules, target, venv, __pycache__, dist, build, .next, vendor, .gradle …]
  └─ → Vec<DevModuleEntry {path,kind,language,size,file_count,drive}> → DevCleaner.tsx grouped by language
Tauri clean_dev_modules / clean_all_dev_modules → DevModulesScanner::delete_modules (protected + known-pattern guard)
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