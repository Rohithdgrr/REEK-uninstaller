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

### Changed
- Fixed cross-platform feature gating (`#[cfg(all(target_os = "windows",
  feature = "windows"))]`) so `--all-features` builds on Linux/macOS.
- Removed unused optional platform bindings (rust-apt/alpm/rpm/plist) that
  broke `--all-features` on non-Linux hosts; scanners shell out to system tools.
- Cross-platform system stats types moved to `greek-common`, re-exported by
  `greek-windows`.
- Made the full quality gate pass: clippy `-D warnings`, rustfmt, and all
  workspace tests across features.

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
