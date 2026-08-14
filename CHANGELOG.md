# Changelog

All notable changes to REEK Ultimate Uninstaller will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Initial project structure with workspace configuration.
- Core data models and error handling (greek-common).
- Cross-platform abstractions (greek-platform).
- Windows-specific implementations (greek-windows).
- Core uninstallation logic (greek-core): standard, MSI, and force-remove
  strategies; leftover analysis; restore-point creation; batch uninstall.
- Command-line interface (greek-cli, binary `reek`).
- Terminal user interface (greek-tui, binary `reek-tui`).
- CI/CD pipeline (`.github/workflows/ci.yml`).
- Security documentation and tooling (SECURITY.md, docs/).

### Security
- Committed `Cargo.lock` for reproducible, audited builds (was gitignored).
- Hardened CI: pinned GitHub Actions to immutable commit SHAs; added
  least-privilege `permissions: contents: read`; added `concurrency` and
  `--locked` on all cargo invocations; added MSRV (1.78.0) and rustdoc jobs.
- Hardened `cargo-deny` config: deny wildcard dependencies, unknown
  registries/licenses; deny unmaintained/yanked workspace deps.
- Added explicit `license` fields and versioned path dependencies to all
  workspace crates (resolves `cargo-deny` license/wildcard failures).
- Quote-aware parsing of uninstall command strings to avoid shell
  metacharacter interpretation; uninstall execution is timeout-bound and runs
  without a shell.
- Restore point created before uninstall by default; failures are non-fatal
  and logged.
- Force-remove now records a rollback-able **backup transaction** before
  deleting anything (files/directories copied, registry keys exported), so an
  uninstall can be undone via `undo_uninstall`. Windows file removal uses the
  OS Recycle Bin (`SHFileOperationW`) instead of a permanent delete.
- Force-remove is disabled in the TUI unless running elevated on Windows, and
  the footer shows `[F]orce (admin)` when privileges are missing.

### Changed
- Fixed cross-platform feature gating (`#[cfg(all(target_os = "windows",
  feature = "windows"))]`) so `--all-features` builds on Linux/macOS.
- Removed unused optional platform bindings (rust-apt/alpm/rpm/plist) that
  broke `--all-features` on non-Linux hosts; scanners shell out to system tools.
- Cross-platform system stats types moved to `greek-common`, re-exported by
  `greek-windows`.
- Made the full quality gate pass: clippy `-D warnings`, rustfmt, and all
  workspace tests across features.
- TUI no longer spawns a fresh `tokio::Runtime` per action; it reuses the
  runtime handle created in `main` (`Handle::spawn`).
- Fixed macOS/Linux-only build errors: `home` shadowing in
  `browser_extensions.rs`, Windows-only `RegistryLeftoverAnalyzer` impl,
  unused import/variable in `task_scheduler.rs`.

### Added (this batch)
- Uninstall transaction backup/rollback (`greek-core/src/backup.rs`) with
  manifest persistence, listing, and `GreekAppService::undo_uninstall`.
- Windows Recycle Bin support (`greek-windows/src/recycle.rs`).
- Windows elevation detection (`greek-windows/src/elevation.rs`) + TUI gating.
- Coverage job in CI (cargo-llvm-cov, pinned 0.8.7) with a 30% regression
  floor and `lcov.info` artifact; `make coverage` targets.

### Fixed
- Deadlocks in rayon-based parallel scans (Mutex removed; per-fork merging).
- WMI non-snake-case row structs, dead code, and test tautologies flagged by
  clippy.
- Windows test assertions that required a live WMI/service provider.

## [0.1.0] - TBD

### Initial Release
- Basic uninstallation functionality.
- CLI and TUI interfaces.
- Windows registry scanning.
- File system cleanup.
