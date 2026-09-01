# REEK Ultimate Uninstaller

The uninstaller that actually uninstalls. A cross-platform application
uninstaller written in pure Rust, with a **Tauri v2 desktop UI** (React + TS),
terminal UI (`reek-tui`) and headless CLI (`reek`). REEK scans for installed
applications, uninstalls them (standard, MSI, or force-remove), does
**whole-device leftover hunting**, and on Windows can create system restore
points before uninstalling.

> **Status:** early development (0.1.x). Expect breaking changes.
> **New in this release:** Movies vault, Dev Cleaner, whole-device leftovers + duplicate-installer detection, auto-update via Tauri updater, and explicit **exclusion of documents/images** from deletion.

## Features

- **Safe application list** — Windows Registry + Store, Linux package managers
  (apt/rpm/flatpak), macOS `.app`. OS-critical and inbox defaults (Clock, Calendar,
  Snipping Tool, Calculator, Photos, etc.) are **hidden by default** — only
  externally-installed, safe-to-remove apps are shown (Office, Chrome, Docker, …).
  Logic: `SystemComponent=1`, `ReleaseType=Update`, VC++/ .NET runtimes, drivers,
  plus `INBOX_DEFAULT` lists with publisher guard (`crates/greek-common/src/constants.rs`).
- **Three uninstall strategies** — standard uninstall string, MSI product-code
  uninstall, and force-remove (kill processes, delete files + registry keys).
- **Whole-device leftover analysis** — **all drives** (`Program Files`,
  `Users\*\AppData`, `ProgramData`, `Windows` shallow) + **registry**
  (`HKLM/HKCU Software, Run, Services, Uninstall, App Paths, Classes`) + **junk/temp**
  (`Windows\Temp`, `%TEMP%`, Prefetch, cache) + **services**, **scheduled tasks**,
  **shortcuts** and **duplicate installers** (`Downloads`, drive roots). Each artifact
  tagged with `SafetyLevel` and accurate folder size. Shown grouped by drive with
  category chips (`Folders`, `Junk`, `Registry`, `Services`, `Tasks`, `Shortcuts`, `Duplicates`).
  **Documents & images are never flagged** — `.pdf, .doc/.docx, .ppt/.pptx, .xls/.xlsx, .jpg/.jpeg, .png, .gif, .webp, .svg, .heic …` are excluded even if filename contains the app token.
  See `crates/greek-core/src/leftover.rs:EXCLUDED_DOC_IMAGE_EXTS`.
- **Duplicate installer detection** — if you downloaded `Cursor-Setup.exe` 3× to
  `Downloads` / `D:\`, it lists every copy across all drives + `Downloads/Desktop/Documents`
  and shows **“Duplicate — safe to delete, does NOT affect installed app at C:\Program Files\Cursor”** (green `Safe` badge). Deleting duplicates never touches `install_location`.
- **Movies / Video vault** — button in dashboard scans **whole device** for videos
  (`.mp4, .mkv, .avi, .mov, .wmv, .flv, .webm …` 34 exts, `>1 MiB`, grouped by drive)
  from `Videos`, `Downloads`, `Desktop`, `C:\Users\*` and every drive (`D:\Movies`).
  List shows size, drive, play, select + **Delete selected** (recycle bin). `crates/greek-core/src/video.rs`.
- **Dev Cleaner — one-click purge** — finds `node_modules`, `target` (Rust/Cargo & Java),
  Python `venv/.venv/__pycache__/.pytest_cache`, `dist/build/out/.next/.nuxt`,
  `vendor`, `.gradle`, `.parcel-cache` etc. across all drives & all users
  (`Documents/Projects/code/dev/workspace`). Grouped by language (`Node`, `Python`, `Rust`…) with file-count & size. **Delete selected** or **Delete ALL** in one tap — safe, recreatable via `npm install / pip install / cargo build`. `crates/greek-core/src/dev_modules.rs`.
- **Success animation** — after deleting any app, video, or dev module, a **UPI-style big green-tick dialog** animates (scale + ripple + confetti, `SuccessTickDialog.tsx`) before returning to the list.
- **System Restore points** (Windows) — created before uninstall by default.
- **Live system stats** (Windows) — CPU/RAM/disk/process/GPU/vRAM metrics in the Tauri header + `SystemStatsBar`.
- **Three front-ends** — Tauri desktop app (1200×800, Mahakali theme), ratatui TUI, and scriptable CLI.
- **Auto-update** — Tauri `plugin-updater` with GitHub Releases as endpoint (`src-tauri/tauri.conf.json:plugins.updater`). CI builds updater artifacts (`createUpdaterArtifacts: true`) and publishes `latest.json`. See `docs/AUTO_UPDATE.md` and `docs/CI_CD.md`.
- **Security-first design** — protected-path/registry guards, quote-aware command parsing, timeouts, pinned deps, `EXCLUDED_DOC_IMAGE_EXTS` never touched. See [SECURITY](SECURITY.md).

## Requirements

- Rust **1.88.0+** (MSRV). No nightly features used.
- Windows builds need the MSVC toolchain (via rustup `x86_64-pc-windows-msvc`).
- Linux: no native bindings required — scanners shell out to `apt`/`rpm`/
  `flatpak`/`brew` when present.

## Workspace

```
crates/
├── greek-common    shared types, errors, constants, traits (+ VideoEntry, DevModuleEntry)
├── greek-core      business logic: scanners, uninstaller, leftovers, config,
│                   video scanner (crates/greek-core/src/video.rs),
│                   dev-modules scanner (dev_modules.rs)
├── greek-windows   Windows capabilities (registry, services, restore, WMI)
├── greek-platform  Linux/macOS scanners + common platform helpers
├── greek-cli       headless CLI binary  → reek
├── greek-tui       terminal UI binary   → reek-tui
└── src-tauri/      Tauri v2 desktop app (React + TS, updater, Movies & Dev Cleaner)
    ├── src/components/VideoVault.tsx, DevCleaner.tsx, SuccessTickDialog.tsx
    └── tauri.conf.json (createUpdaterArtifacts + updater endpoints)
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/AUTO_UPDATE.md](docs/AUTO_UPDATE.md).

### What is NEVER deleted (safety)

Documents & images are **never** flagged as leftovers/junk/duplicates, even if the filename contains the app name. Excluded extensions (`crates/greek-core/src/leftover.rs:EXCLUDED_DOC_IMAGE_EXTS`):

`pdf, doc, docx, ppt, pptx, xls, xlsx, csv, txt, rtf, odt, jpg, jpeg, png, gif, bmp, webp, svg, heic…`

The Movies vault is **opt-in** (separate button) and only lists video extensions (`mp4, mkv, avi, mov, wmv, flv, webm…` 34 types) from `video.rs:VIDEO_EXTS`. Your reports, presentations, and photos are left untouched.

## Build & run

```bash
# Build everything (Rust)
cargo build --workspace

# Frontend + Tauri desktop (auto-update enabled)
npm ci
npm run build          # Vite build (checks TS)
npm run tauri dev      # Tauri dev (http://localhost:1420)
npm run tauri build    # Tauri release + updater artifacts (src-tauri/target/release/bundle + latest.json)

# Run the TUI
cargo run --bin reek-tui

# Run the CLI
cargo run --bin reek -- --help
```

### Desktop app — Movies & Dev Cleaner

* **Movies** button (dashboard) → `VideoVault` scans every drive (`Videos`, `Downloads`, `Desktop`, `C:\Users\*`, plus `D:\Movies`) for videos, grouped by drive with play & delete (recycle bin). Trigger via Tauri `scan_videos` / `delete_videos`.
* **Dev Cleaner** button → `DevCleaner` finds `node_modules`, `.venv`, `target`, `dist`, `build`, `.next`, `vendor`, `__pycache__` etc. across all drives & users, grouped by language (`Node`, `Python`, `Rust` …) with file-count & size. `Delete selected` or `Delete ALL` (one tap) via `scan_dev_modules` / `clean_dev_modules`.
* After any delete (app, video, dev module) a **UPI-style green-tick dialog** animates (`SuccessTickDialog.tsx` — scale+ripple+confetti) before returning to the list.

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

## Observability

- Structured logs: JSON to `<data_dir>/logs/reek.log.YYYY-MM-DD` (daily rotation, 14-day retention), pretty to `stderr`. `RUST_LOG=debug` or `--verbose` controls level. Secrets redacted (`sanitize_output` 8KB cap).
- See [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md) — log locations, how to file a bug, opt-in metrics/crash reporting.

## Security

- [SECURITY.md](SECURITY.md) — threat model, supported versions, vulnerability
  reporting.
- [docs/SECURITY.md](docs/SECURITY.md) — layered security architecture.
- Security tooling: `cargo-audit`, `cargo-deny`, committed `Cargo.lock`, pinned
  CI action SHAs, least-privilege CI permissions, SBOM via `cargo auditable`.

## Documentation

| Doc | Contents |
|-----|----------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Crate layout, data flow, feature gates, new modules (video, dev) |
| [docs/CI_CD.md](docs/CI_CD.md) | CI jobs, gates, security, Tauri build, auto-update pipeline |
| [docs/AUTO_UPDATE.md](docs/AUTO_UPDATE.md) | Tauri updater setup, signing, endpoints, testing locally |
| [docs/RELEASING.md](docs/RELEASING.md) | Release process & checklist (cargo + Tauri) |
| [docs/SECURITY.md](docs/SECURITY.md) | Security architecture & controls |
| [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md) | Logs, retention, metrics, crash reporting |
| [INSTALL.md](INSTALL.md) | Installing the Rust toolchain & building |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution guide |
| [CHANGELOG.md](CHANGELOG.md) | Notable changes |
| [PRD.md](PRD.md), [features.md](features.md) | Product requirements & feature spec |

## License

MIT OR Apache-2.0. See [LICENSE](LICENSE).

## Support

- Security issues: see [SECURITY.md](SECURITY.md) — do **not** use public issues.
- Bugs & features: open a GitHub issue or PR per [CONTRIBUTING.md](CONTRIBUTING.md).