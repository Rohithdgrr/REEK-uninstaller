# REEK Ultimate Uninstaller — Project Audit Report

**Date:** 2026-09-01  
**Version audited:** `0.1.0` (workspace `resolver = 2`, MSRV `1.88.0`)  
**Scope:** All crates (`greek-common`, `greek-core`, `greek-windows`, `greek-platform`, `greek-cli`, `greek-tui`), root configs (`Cargo.toml`, `deny.toml`, `audit.toml`, `clippy.toml`, `rustfmt.toml`, `Makefile`), CI (`.github/workflows/ci.yml`), docs (`PRD.md`, `features.md`, `docs/*.md`)  
**Method:** Static-code review + live toolchain verification:

- `cargo audit` → 1 vulnerability (`h2` RUSTSEC-2026-0258), 4 warnings (`number_prefix`, `paste`, `lru`×2)
- `cargo deny check advisories bans licenses sources` → 4 unmatched license allows + duplicate crates (`core-foundation`×2, `crossterm`×2, `getrandom`×2, `hashbrown`×3)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → **build failure** on `greek-platform/src/common.rs:21,26` (`unexpected cfg feature = "greek-windows"`)
- Manual review of ~40 `*.rs` files (≈ 8000 LOC)

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Critical Bugs (Fix Now — Data Loss / Brick Risk)](#2-critical-bugs-fix-now--data-loss--brick-risk)
3. [High Severity — Functional / Correctness Bugs](#3-high-severity--functional--correctness-bugs)
4. [Medium Severity — Reliability & Tech Debt](#4-medium-severity--reliability--tech-debt)
5. [Low Severity — Code Quality & Maintainability](#5-low-severity--code-quality--maintainability)
6. [Risks & Threat Model Gaps](#6-risks--threat-model-gaps)
7. [Stability & Efficiency Improvements](#7-stability--efficiency-improvements)
8. [Performance Improvements](#8-performance-improvements)
9. [Security Improvements](#9-security-improvements)
10. [Dependency & Supply-Chain Health](#10-dependency--supply-chain-health)
11. [CI/CD & Process Improvements](#11-cicd--process-improvements)
12. [Feature Coverage vs PRD/features.md](#12-feature-coverage-vs-prdfeaturesmd)
13. [Prioritized Remediation Roadmap](#13-prioritized-remediation-roadmap)
14. [Appendix — Verification Evidence](#14-appendix--verification-evidence)

---

## 1. Executive Summary

The workspace is well-structured, security-conscious in design (no-shell execution, timeouts, pinned actions, `Cargo.lock` committed), and partially implements PRD P0 features. **However, the current main branch does not pass `clippy -D warnings` and ships a known `h2` vulnerability.** Three critical logic bugs would cause incorrect protection decisions on Linux/macOS and silent data loss in the CLI. ~55% of PRD P0 and ~75% of P1 features are missing or stubbed.

Overall grade: **C+ (prototype quality)**. With Phase 0 fixes it becomes **B (shippable internal tool)**; with Phase 1+2 it reaches **A- (production-grade)**.

---

## 2. Critical Bugs (Fix Now — Data Loss / Brick Risk)

### 2.1 `PROTECTED_PATHS` blocks every Unix path

- **File:** `crates/greek-common/src/constants.rs:72-73`
- **Code:**
  ```rust
  pub const PROTECTED_PATHS: &[&str] = &[ "/","/bin","/sbin", ... "/opt", "/System", "/Library", ... ];
  ```
- **Impact:** Entry `"/"` at line `73` is a prefix of every absolute path. `is_protected_path()` (`crates/greek-common/src/lib.rs:27`, `crates/greek-core/src/utils.rs:34`) does `path_str.starts_with("/")` → **every** install location on Linux/macOS is considered protected, so `ForceRemove` will either refuse legitimate deletes or, if the check is inverted elsewhere, the broad `/` entry encourages developers to ignore the guard.
- **Secondary:** Same prefix without separator guard. `C:\Windows` also blocks `C:\WindowsAppsFoo` (`lib.rs:31`).
- **Fix:**
  1. Remove `"/"` and entries that are strict prefixes of broader ones unless intentional. Add `"/"` only as explicit `Path::parent == None` check.
  2. Replace `starts_with` with canonicalize + `strip_prefix` or `path == protected || path.starts_with(protected + MAIN_SEPARATOR)`.
  3. Normalize case + trailing separators, call `std::fs::canonicalize` where path exists.

### 2.2 `EstimatedSize` registry value always `None`

- **File:** `crates/greek-windows/src/registry.rs:155-157`
- **Code:** `get_value::<String,_>("EstimatedSize")?.parse::<u64>()`
- **Problem:** Windows stores `EstimatedSize` as `REG_DWORD` (kilobytes), not `REG_SZ`. The string parse always fails, so `size_bytes` and sort-by-size are broken. Also misses `REG_SZ "1"` form for `is_system_component` at `registry.rs:160`.
- **Fix:** Try `get_value::<u32,_>` first, fallback to `String` → `u32`, multiply by `1024`. For `SystemComponent`, accept both `u32 ==1` and `String == "1"`.

### 2.3 `services.rs` ProcessId always zero

- **File:** `crates/greek-windows/src/services.rs:53-56`
- **Code:** `Select-Object Name, DisplayName, Status, @{N='ProcessId';E={0}}`
- **Impact:** `find_services_for_app()` and any service-to-process linking is useless; `kill_processes_by_path` may miss services that host processes.
- **Fix:** Query `Get-CimInstance Win32_Service | Select Name, DisplayName, State, ProcessId` or `Get-WmiObject`-based query. Existing `wmi.rs:16` already has `Get-CimInstance` helper — reuse it. Also match services by `ImagePath` prefix, not just name substring `services.rs:42`.

### 2.4 `scanner.rs:196-214` Parallel scan is dead code

- **File:** `crates/greek-core/src/scanner.rs:196-278`
- **Code:** `par_iter().filter_map(|dir| None::<Vec<InstalledApp>>)` and unused `scanners` variable.
- **Impact:** The intended `rayon` parallel portable scan never runs. All scans are sequential `scanner.rs:97`.
- **Fix:** Delete placeholder, implement `scan_all()` with `futures::future::join_all` for async scanners and `rayon` + `spawn_blocking` for file-system walks. Merge results and deduplicate via `deduplicate_apps()`.

### 2.5 CLI `--output` / `--export` never writes files

- **Files:** `crates/greek-cli/src/main.rs:193-196` (`cmd_list`), `296-339` (`cmd_scan`)
- **Impact:** `reek list --output apps.csv` and `reek scan --export leftovers.json` only print `"Output saved to: {}"` without creating a file — silent user-data loss expectation.
- **Fix:** Add `std::fs::write(output_path, contents)` for each format (`table`/`json`/`csv`); propagate I/O error via `color_eyre`.

### 2.6 Registry path parser has unreachable branch

- **File:** `crates/greek-core/src/utils.rs:159-165`
- **Impact:** Double-escaped `strip_prefix("HKLM\\\\")` vs `"HKLM\\\\\\\\"` — second branch unreachable; `Hkcu` Wow64 path handling incomplete.
- **Fix:** Single-pass parser: `hive, rest = path.split_once('\\').unwrap()`, normalize `\\` → `\`, then match `HKLM`/`HKCU`/`HKCR`/`HKU`. Consolidate with `registry.rs:46` logic and use `KEY_WOW64_32KEY` flags instead of separate paths.

### 2.7 Platform `is_elevated()` always false on Windows

- **File:** `crates/greek-platform/src/common.rs:21-38`, `crates/greek-platform/Cargo.toml`
- **Impact:** `#[cfg(all(target_os="windows", feature="greek-windows"))]` requires a feature that `greek-platform` never declares → clippy error and fallback `false`. `GreekAppService` elevation gate for `ForceRemove` (`crates/greek-tui/src/app.rs:284`) never triggers correctly when called via core.
- **Fix:** Declare `[features] greek-windows = ["dep:greek-windows"]` in `greek-platform/Cargo.toml` or replace with `#[cfg(target_os="windows")]` + runtime `greek_windows::is_elevated()` call gated by `cfg`.

### 2.8 TUI selection desynchronizes after filter/sort

- **File:** `crates/greek-tui/src/app.rs:118-142`, `499-505`, `618-637`
- **Impact:** `selected_apps: Vec<usize>` stores indices into `apps`/`filtered_apps`. After `filter_apps()` or sort, indices point to wrong items; batch uninstall may remove the wrong application.
- **Fix:** Store `HashSet<Uuid>` as spec `features.md:7.2` already describes. Resolve indices to `Uuid` at selection time.

---

## 3. High Severity — Functional / Correctness Bugs

| # | File:Line | Issue | Fix |
|---|-----------|-------|-----|
| H1 | `crates/greek-core/src/utils.rs:26-31` | Prefix without separator guard blocks `C:\WindowsFoo`. | Require `path == protected || path.starts_with(protected + '\')` (Windows) or `'/'` (Unix) after lowercasing + canonicalize. |
| H2 | `crates/greek-core/src/utils.rs:43-51` | `PROTECTED_REGISTRY_PATHS` prefix `HKLM\SOFTWARE\Microsoft\Windows` blocks `HKLM\SOFTWARE\Microsoft\WindowsAppsFoo`. | Same separator guard (`\` or end-of-string). |
| H3 | `crates/greek-core/src/uninstaller.rs:389-412` | `Regex::new` compiled on every `MsiUninstallStrategy::extract_product_code` call. | Use `OnceLock<Regex>` static. |
| H4 | `crates/greek-core/src/uninstaller.rs:295` | `can_handle` checks `contains("MsiExec")` case-sensitive, misses `msiexec`. | Use `to_ascii_lowercase().contains("msiexec")`. |
| H5 | `crates/greek-core/src/uninstaller.rs:545-565` | `kill_processes_by_path` kills any `exe_path.starts_with(path_str)` — prefix collision `C:\App` kills `C:\App2`. | Canonicalize both paths, require separator guard, compare `Path::starts_with`. |
| H6 | `crates/greek-core/src/leftover.rs:189-234` | `scan_for_orphans` flags any dir `age >30d` as orphan with low confidence `0.3` → false positives on user docs. | Require keyword match (app name/publisher) **and** age check; do not emit pure age-based artifacts. |
| H7 | `crates/greek-core/src/leftover.rs:147-187` | `path_str.contains(app_name)` substring — `"app"` matches `"application"`. | Use word-boundary or case-insensitive `contains` with length guard `app_name.len() >= 4` + path component tokenization. |
| H8 | `crates/greek-core/src/leftover.rs:261-301` | `RegistryLeftoverAnalyzer::analyze` returns `Ok(Vec::new())` — PRD §3.3 deep scan missing. | Implement: enumerate `HKCU\Software`, `HKLM\Software`, check orphan keys vs installed list. |
| H9 | `crates/greek-core/src/backup.rs:246-261` | `copy_tree` recurses without symlink handling → loop or target copy. | Use `symlink_metadata`, skip symlinks or reproduce as symlink, add depth limit. |
| H10 | `crates/greek-core/src/backup.rs:266-278` | `restore_path` refuses overwrite if `original.exists()` → incomplete rollback if app recreated dir. | Use atomic `rename` + temp backup, or `fs::remove_dir_all` before restore if `force`. |
| H11 | `crates/greek-core/src/task_scheduler.rs:69-102` | Single-task PS output is JSON object, not array → `from_str::<Vec<_>>` fails. | Accept `serde_json::Value`, handle `Array` and `Object` cases. |
| H12 | `crates/greek-windows/src/restore.rs:170-191` | `run_powershell` masks errors: `if !stderr.contains("not recognized") return Ok` hides real failures. | Check `status.success()` + `exit_code`; log `stderr` explicitly. |
| H13 | `crates/greek-windows/src/store.rs:67-117` | `is_framework` heuristic `Microsoft.` + `UI/Framework` misses `VCLibs`, false positives. | Use `IsFramework` property from manifest or `PackageFamilyName` suffix. |
| H14 | `crates/greek-platform/src/linux.rs:320-404` | `sudo` may prompt for password and hang TUI indefinitely. | Use `sudo -n` (non-interactive) + timeout, pre-check `is_elevated()`/`sudo -v`, surface clear error. |
| H15 | `crates/greek-platform/src/macos.rs:397-437` | `sudo rm -rf` with no trash, no confirm, crosses filesystems, destructive. | Use `trash` crate or `NSFileManager.trashItemAtURL`. Keep `rm -rf` only behind `--force` + protected-path re-check. |
| H16 | `crates/greek-core/src/browser_extensions.rs:82-228` | `let _home = var("HOME").ok()?` returns `None` on Windows → early exit even though `LOCALAPPDATA` path exists. | Move `HOME` lookup inside each `#[cfg]` branch. |
| H17 | `crates/greek-windows/src/wmi.rs:68` | `Win32_Product` triggers Windows Installer repair scan (notoriously slow, reconfigures apps). | Query `Win32_InstalledWin32Program` or registry instead; avoid `Win32_Product` entirely per Microsoft guidance. |
| H18 | `crates/greek-tui/src/ui.rs:360-368` | `truncate` uses `s.len()` bytes not chars → breaks UTF-8, panic on `truncate` boundary. | Use `unicode-width` / `char` boundary-aware truncation. |
| H19 | `crates/greek-cli/src/main.rs:201-225` | `cmd_search --fuzzy` does `contains` again, not using `fuzzy-matcher`; exact mode uses `==` case-insensitive while TUI uses `SkimMatcherV2`. | Unify: use `fuzzy-matcher` in CLI when `--fuzzy`, otherwise `contains`. |
| H20 | `crates/greek-cli/src/main.rs:341-389` | `cmd_clean` hardcodes `UninstallOptions::force()` regardless of user flags, bypasses `SafetyLevel` filter. | Respect config `safety` + `leftover.confidence_threshold`, require explicit `--force` for non-`Safe`. |

---

## 4. Medium Severity — Reliability & Tech Debt

### 4.1 Error Handling

- **File:** `crates/greek-common/src/error.rs:1-173`
- `GreekError::ScanError(String)` erases source chain via `to_string()`, loses `#[source]` for `color-eyre`. Variants `Timeout(String)` vs `UninstallError::Timeout(u64)` duplicate.
- **Fix:** Use `#[source] ScanError` transparent or `thiserror` `#[from]` with `Box<dyn Error>`, keep `UninstallError::Timeout` as single source.

### 4.2 `is_protected_path` duplication

- **Files:** `crates/greek-common/src/lib.rs:26`, `crates/greek-core/src/utils.rs:34`
- Two identical implementations → drift risk. Keep canonical in `greek-common`, re-export.

### 4.3 `clean_publisher_name` X.500 fragility

- **File:** `crates/greek-common/src/models.rs:11-58`
- Splits on `,` ignoring escaped commas (`CN=Foo\, Bar`). Truncates `"Foo, Inc."` → `"Foo"` when `len>=3`.
- **Fix:** Use single separator loop with escape handling; only truncate when `publisher.contains(" (")` etc after X.500 parse fails.

### 4.4 Cache & Scanning

- **File:** `crates/greek-core/src/app_service.rs:120,241`
- `scan_all_apps(&mut self)` needs `&mut` for `app_cache` (TTL 30s) but `uninstall_app(&self)` is `&self` — borrow asymmetry forces `Mutex` or `RwLock`. `get_app_details` does full `scan_all_apps` O(n) each call.
- **Fix:** Change caches to `RwLock<HashMap>` or `DashMap`; add `get_app(id)` that hits cache first.

### 4.5 `parse_command_string` coverage

- **File:** `crates/greek-core/src/uninstaller.rs:211-259`
- Hand-rolled quote parser handles `""` escaped quotes but misses `^` escapes (cmd.exe) and trailing `\` before `"`.
- **Fix:** Keep `shell_words::split` as primary, augment with `^` handling for Windows, add exhaustive unit tests with real `UninstallString` samples.

### 4.6 `ConfigManager::default()` panics

- **File:** `crates/greek-core/src/config.rs:118` `expect("valid default")`
- **Fix:** Return `Self` directly or `Result<Self>`, never panic in lib.

### 4.7 `scan_portable_dirs` validation only warns

- **File:** `crates/greek-core/src/config.rs:78-81` `tracing::warn` even when configured path missing.
- **Fix:** Filter non-existent dirs, or surface as `ConfigError`.

### 4.8 `PortableAppScanner` misses apps where exe name differs

- **File:** `crates/greek-core/src/scanner.rs:416-456`
- Only matches `exe_stem == dir_name` inside top-level dir, depth 1.
- **Fix:** Scan one level deeper, match against `displayName` heuristics, SHA256 dedup, respect `scanner.scan_portable_dirs` config.

### 4.9 Windows hive split misses Wow64 for HKCU

- **File:** `crates/greek-windows/src/registry.rs:46-96`
- Only scans `HKLM\...Wow6432Node`, not `HKCU\...Wow6432Node`. Should scan via `KEY_WOW64_32KEY | KEY_WOW64_64KEY` flags.
- **Fix:** Open key once with each flag rather than separate path strings.

### 4.10 `app_service.rs:260-290` leftover cache never evicted

- **File:** `crates/greek-core/src/app_service.rs:34,260`
- `artifact_cache: HashMap<Uuid, Vec<LeftoverArtifact>>` grows without TTL; `list_transactions` sorts timestamps as string `backup.rs:207` (works for RFC3339 but parse as `DateTime` safer).
- **Fix:** Add `DashMap` with TTL or `lru` with capacity, or clear on `clean_leftovers`.

### 4.11 `system_stats.rs:70-159` blocks thread

- **File:** `crates/greek-windows/src/system_stats.rs:73` `sleep 250ms` for CPU delta inside `collect(&mut self)`. Works because spawned in `TuiApp::new` thread, but blocks `tokio` if called elsewhere.
- **Fix:** Make `collect` async or pre-warm with two samples, document blocking.

### 4.12 `recycle.rs:37` deprecated API, `elevation.rs:13` deprecated check

- **Files:** `crates/greek-windows/src/recycle.rs:37` (`SHFileOperationW` deprecated since Vista), `crates/greek-windows/src/elevation.rs:13` (`IsUserAnAdmin` deprecated)
- **Fix:** `IFileOperation` + `CheckTokenMembership` / `TokenElevationType`.

### 4.13 TUI event loop dead code

- **File:** `crates/greek-tui/src/events.rs:1-92` (`EventHandler` with `mpsc::Unbounded`) vs `crates/greek-tui/src/app.rs:383` sync `crossterm::event::poll` loop.
- **Fix:** Remove `events.rs` or wire it as sole event source; current dead code increases surface.

### 4.14 CLI `verbose`/`json` flags unused

- **File:** `crates/greek-cli/src/main.rs:18-31`
- Declared `verbose,bool json` but never wired to `EnvFilter` or output formatting.
- **Fix:** Init `tracing_subscriber::fmt().with_env_filter` when `verbose`; branch all `println!` through `json` flag.

### 4.15 `ScannerManager` not registering existing scanners

- **Files:** `browser_extensions.rs`, `windows_features.rs`, `task_scheduler.rs`, `wmi.rs` exist but not in `ScannerManager::new()`.
- **Fix:** Register adapters for each, or explicitly document as separate `analyze_leftovers` path.

---

## 5. Low Severity — Code Quality & Maintainability

- `clippy.toml:1` `cognitive-complexity = 30` (default 25) masks large fns (`uninstaller.rs:211` 50 lines, `macos.rs:397`); lower to 25 and refactor.
- `rustfmt.toml` well-configured — keep.
- `Model` duplicates: `SystemStats/DiskStat/GpuStat/BatteryStat/ProcessUsage` defined in `greek-common` and re-exported via `greek-windows/src/system_stats.rs:16`; keep single source (`greek-common`).
- `traits.rs:1-161` over-engineered: `BackupManager/RestorePointManager/ProcessManager/ServiceManager/TaskManager` never implemented; prune or implement.
- Coverage floor `20%` (`ci.yml:192`) reflects Windows-gated code excluded on Linux — add `#[cfg]` coverage splits or raise per-crate floor.
- `deny.toml:23,25,26,30` unmatched licenses `BSD-2-Clause`, `ISC`, `Unicode-DFS-2016`, `OpenSSL` — remove to reduce warnings.
- `Makefile:1` binary names `reek` vs docs `greek`; add alias.
- `app.rs:383` keybind clash: `a`/`Ctrl+a` select all, `n` clear, missing `s` sort/`f` filter per `PRD §5.2`, `?`/`h` both help — normalize to PRD table.
- `theme.rs:164` only handles `#RRGGBB` + `white/black/grey`; add `#RGB`, `rgba()`.
- `widgets.rs:1-103` unused `TextWidget/StatusBarWidget`; remove or consume in `ui.rs`.

---

## 6. Risks & Threat Model Gaps

| Risk | Impact | Current Mitigation | Residual | Recommended Action |
|------|--------|-------------------|----------|--------------------|
| `UninstallString` executes arbitrary exe | Critical | No shell, timeout 300s `greek-core/src/uninstaller.rs:185`, but no signature check | High | Verify Authenticode signature before exec; warn for unsigned + show full command. File: `uninstaller.rs:121` |
| PowerShell injection via `app.name`/`feature_name`/`TaskPath` | High | Single-quote escape `replace('\'',"''")` `store.rs:278`, `services.rs:114` but not inside `" "` `windows_features.rs:68` | Medium | Centralize `escape_ps` helper, test with `O'Reilly` / `"`. Add fuzz tests. |
| Path traversal / `..` bypass of `is_protected_path` | High | `starts_with` lowercased only `lib.rs:27` | Medium | Canonicalize + reject `..` components before check `utils.rs:34` |
| Force-remove kills wrong processes | Medium | `starts_with` prefix `uninstaller.rs:545` | Medium | Canonical + separator guard, kill by PID not name, confirm list to user. |
| `Win32_Product` repair storm | Medium | Used `wmi.rs:68` | Medium | Avoid entirely; use registry. |
| Non-Windows recycle fallback deletes permanently | Medium | `utils.rs:108` fallback `delete_directory` | Medium | `trash` crate fallback; otherwise backup/undo remains `backup.rs:77` |
| Backup accumulation no TTL | Low | `backups/<uuid>/` `backup.rs:55` | Low | Add retention (7 days / 500 MB) + `reek clean --prune-backups`. Docs note `docs/SECURITY.md:113` — implement. |
| Restore point non-fatal continue without user ack | Low | Logged warn `app_service.rs:220` | Low | Surface toast + require explicit `--ignore-restore-failure` in CLI. |

---

## 7. Stability & Efficiency Improvements

### 7.1 Stability

1. **Canonicalize all paths** before protection checks and deletions (`utils.rs:98` `delete_directory`, `backup.rs:77` `add_file_or_dir`, `uninstaller.rs:443` `ForceRemove`). Use `normalize_path()` that handles `\\?\`, trailing slashes, junctions.
2. **Make `GreekAppService` caches thread-safe** (`RwLock`/`DashMap`) so `scan_all_apps(&self)` not `&mut self` — fixes borrow asymmetry `app_service.rs:120` and enables concurrent CLI+TUI.
3. **Add circuit breakers for external CLIs:** wrap `powershell.exe`, `dism.exe`, `dpkg-query`, `plutil` with `tokio::time::timeout` + retry + `which/exists` pre-check. Current `windows_features.rs:68` launches 3 sequential `powershell.exe` without timeout.
4. **Transaction manifest integrity:** write `manifest.json` atomically (`write temp → rename`) and validate JSON schema on load `backup.rs:207`.
5. **Undo queue persistence:** current `undo_uninstall` `app_service.rs:386` only in-memory; persist to `backups/manifest.json` + SQLite journal `features.md:50`.
6. **Fix clippy failure gate:** add missing `[features]` in `greek-platform/Cargo.toml` so `cargo clippy -D warnings` passes on all targets — blocking CI `ci.yml:51`.

### 7.2 Efficiency

1. **Trim `tokio` features** `Cargo.toml:22` `full` → `["rt-multi-thread","macros","sync","time","process","io-util"]` — halves compile time and binary size (~15 MB from unused `net`, `reqwest` pulls `hyper` etc).
2. **Replace `System::new_all()` + `refresh_all()` per kill** `uninstaller.rs:545` with single instance reused via `OnceLock` + `refresh_processes()`.
3. **Cache PowerShell version check:** `icon.rs:362` `run_ps_script` polls `try_wait` + `100ms` sleep — use `wait_timeout` crate or `async_process`.
4. **Avoid `du -sh` human parse** `macos.rs:236` (`1.2G` lossy) → `du -sk` (KB) for precision.
5. **Batch scans sequentially** `app_service.rs:403` — add optional parallel batch with semaphore `N=2` for I/O overlap.

---

## 8. Performance Improvements

| Area | Current | Target | File | Action |
|------|---------|--------|------|--------|
| Parallel scanning | Sequential `scanner.rs:97` | `join_all` + `rayon` | `scanner.rs:51` | Spawn per-scanner `tokio::spawn`, merge + dedup |
| MSI regex | Compile per-call | `OnceLock` | `uninstaller.rs:389` | Static `Regex` |
| Leftover file walk | `WalkDir max_depth 3` but no ignore, `get_directory_size` unbounded `utils.rs:70` | Bounded | `leftover.rs:147`, `utils.rs:70` | Add `MAX_SCAN_DEPTH`, `MAX_TOTAL_SCAN_SIZE_BYTES` guard, skip `node_modules/.git` |
| System stats | `sleep 250ms` blocking + `Get-Counter` PS per collect `system_stats.rs:70,179` | Async | `system_stats.rs:70` | Pre-warm CPU, cache GPU counter handle |
| Icon extraction | `120+` PS shell jobs chunked `24000` `icon.rs:17,340` | Good already | `icon.rs:126` | Keep; add `spawn_blocking` so not on tokio main thread |
| CLI search | `contains` fallback | `SkimMatcherV2` unified | `cli/main.rs:201` | Share `fuzzy-matcher` with TUI |
| Config TOML | `validate_config` `config.rs:68` string cmp for timestamps `backup.rs:207` | Parse | `config.rs:68` | `chrono::DateTime::parse_from_rfc3339` for sort |

Benchmark hooks: wire `criterion` already in `workspace.dependencies` but unused — add `benches/scan.rs`, `benches/uninstall_parse.rs`.

---

## 9. Security Improvements

### 9.1 Immediate

1. **Upgrade `h2 0.4.15 → 0.4.16`** — `RUSTSEC-2026-0258` unbounded empty DATA frames. Run `cargo update -p h2 --precise 0.4.16`. Verified via `cargo audit`.
2. **Remove `reqwest 0.12.28`** if unused (grep shows zero `use reqwest`). It pulls `openssl`, `hyper`, `h2` (vuln). If needed for `Cloud Sync` `features.md:2.1`, gate behind `feature = "cloud"` default off. File: `crates/greek-core/Cargo.toml:72`.
3. **PowerShell hardening:** Introduce `crates/greek-core/src/ps_escape.rs`:
   ```rust
   pub fn escape_ps_single(s: &str) -> String { s.replace('\'', "''") }
   pub fn ps_arg(p: &str) -> String { format!("'{}'", escape_ps_single(p)) }
   ```
   Use everywhere `store.rs:274`, `services.rs:135`, `windows_features.rs:68`, `restore.rs:15`, `task_scheduler.rs:254`.
4. **Command injection surface:** audit every `Command::new(powershell.exe)` — already no shell `uninstaller.rs:185` good; extend timeout + `kill_on_drop` to PowerShell as well.
5. **Audit log redaction:** current `<redacted>` `utils.rs:152` good for registry path but `UninstallResult.stdout/stderr` `models.rs:332` may contain tokens — cap at `4KB` + redact known patterns.

### 9.2 Hardening

- **Protected paths:** after fix §2.1, add integration tests: `assert!(is_protected_path("C:\\Windows\\..\\Users\\foo", &PROTECTED))`, `assert!(!is_protected_path("C:\\WindowsAppsFoo", &PROTECTED))`.
- **Elevation:** replace `IsUserAnAdmin` `elevation.rs:13` with `unsafe { CheckTokenMembership(None, &sid, &mut is_admin) }` + `TokenElevation`.
- **Recycle bin:** `recycle.rs:37` `SHFileOperationW` → `IFileOperation::MoveToRecycleBin` (Vista+), avoids `FOF_ALLOWUNDO` truncation `u16` flag issue.
- **Dependency bans:** keep `deny.toml:45 multiple-versions = "warn"` — current `hashbrown`×3, `getrandom`×2 etc reduce via `cargo update` and `version` alignment (e.g., `crossterm 0.28` vs `0.29` from `comfy-table 7.2.2`).
- **Pin `windows` crate:** `0.58 → 0.62` — audit changelog for `RIDL` fix; pin via `cargo update -p windows`.

### 9.3 Process

- Keep CI pinning SHAs `ci.yml:32`, `persist-credentials: false`, `permissions: contents: read` — exemplary.
- Switch `audit.toml` report `audit-report.txt` is not `.gitignore`'d — ignore or move to `target/`.
- Add `cargo vet` or `cargo deny check advisories` pre-commit.

---

## 10. Dependency & Supply-Chain Health

**Live findings:**

- **Vulnerability:** `h2 0.4.15` → `0.4.16` (RUSTSEC-2026-0258).
- **Unmaintained:** `number_prefix 0.4.0` (RUSTSEC-2025-0119), `paste 1.0.15` (RUSTSEC-2024-0436) — transitive via `humansize`/`ratatui` pipeline; acceptable if pinned but track replacements. `lru 0.12.5` unsound (RUSTSEC-2026-0002/0253) via `ratatui 0.28.1` — upgrade `ratatui` when fix lands or vendor `lru`.
- **Duplicates to deduplicate:** `crossterm 0.28.1` (TUI) vs `0.29.0` (via `comfy-table` in CLI). Align `comfy-table` version or downgrade CLI dep. `hashbrown 0.14/0.15/0.17`, `getrandom 0.2/0.4`, `core-foundation 0.9/0.10` etc add compile time.
- **Unused:** `reqwest`, `criterion` workspace deps — remove.
- **Stale:** `cargo-llvm-cov 0.8.7` (`ci.yml:175`, current 0.13), `windows 0.58.0` (current 0.62).

**Actions:**

1. `cargo update -p h2 -p windows -p crossterm -p hashbrown -p getrandom`
2. Prune `reqwest` or feature-gate it.
3. Remove `deny.toml` unmatched licenses (`BSD-2-Clause`, `ISC`, etc) or document why needed.
4. Commit deduplicated `Cargo.lock`.

---

## 11. CI/CD & Process Improvements

**Current CI `.github/workflows/ci.yml:1-218` good:** pinned SHAs, `--locked`, matrix `ubuntu/windows/macos`, `concurrency.cancel`.

**Gaps:**

1. **Clippy fails** due to `greek-platform` cfg — fix feature `Cargo.toml` before re-enabling `-D warnings` gate.
2. **`cargo audit` slow** `ci.yml:98` `cargo install --locked` — use `cargo-binstall` or `rustsec/audit-check` action as comment in `docs/CI_CD.md:66` notes.
3. **`coverage` stale** `0.8.7` — bump to `0.13`; raise floor from `20%` gradually to `40%` once Windows-gated code split.
4. **`doc` intra-link check** `ci.yml:218` fragile `grep warning: unresolved` — use `cargo doc --document-private-items` + `RUSTDOCFLAGS="-D warnings"`.
5. **`dependabot.yml:1-22` groups `github-actions` correctly** — add `cargo` grouping for `Cargo.lock` PRs.
6. **Binary name drift:** `Makefile` `run-cli` uses `reek` but docs say `greek` — standardize on `reek` and alias `greek` via `[[bin]] name`.

---

## 12. Feature Coverage vs PRD/features.md

| ID | Feature (Priority) | Status | File |
|----|--------------------|--------|------|
| 1 | Registry Scanner P0 | ✅ Partial | `registry.rs:46` missing Wow64 via flags, `HKCR\Installer` cross-ref |
| 2 | Store Scanner P0 | ✅ | `store.rs:29` |
| 3 | Portable Detection P0 | ⚠️ Partial | `scanner.rs:342` heuristic too narrow |
| 4 | Browser Extensions P1 | ⚠️ Code exists, not wired | `browser_extensions.rs:1` not in `ScannerManager` |
| 5 | Windows Features P1 | ⚠️ Not wired | `windows_features.rs:1` |
| 6 | Startup Items P1 | ❌ Missing | Only `wmi::query_startup_items` unused |
| 7 | Driver Inventory P2 | ❌ Missing | — |
| 8 | Service Mapper P2 | ⚠️ Partial | `services.rs:42` PID bug |
| 11 | Standard Uninstall P0 | ✅ | `uninstaller.rs:84` |
| 12 | Silent Uninstall P0 | ⚠️ Partial | No flag mapping, uses `quiet_uninstall_string` only |
| 13 | Force Remove P0 | ✅ | `uninstaller.rs:443` |
| 14 | Batch Queue P0 | ⚠️ Service partial | `app_service.rs:403` no deps, no TUI queue |
| 21-22 | File/Registry orphans P0 | ⚠️ File yes, Registry placeholder | `leftover.rs:261` |
| 28 | Confidence Scoring P2 | ✅ Heuristic only | `leftover.rs:108` additive 0.4+0.3+… no ML |
| 31 | Dual Pane P0 | ✅ | `tui/src/ui.rs:123` but no tree view |
| 32 | Fuzzy Search P0 | ✅ | `tui/src/app.rs:618` SkimMatcher |
| 34 | Multi-Select P0 | ⚠️ Index bug | `app.rs:499` |
| 35 | Sort P0 | ❌ Missing | `s` not bound |
| 36 | Preview/DryRun P0 | ❌ Missing | `p` not wired |
| 37 | Progress+ETA P0 | ⚠️ Overlay only | `ui.rs:726` |
| 45 | Restore Point P0 | ✅ | `app_service.rs:220` + `restore.rs:15` |
| 46 | Registry Backup P0 | ✅ Win only | `backup.rs:126` |
| 49 | Recycle Bin P1 | ⚠️ Win ok, non-Win deletes | `utils.rs:108` |
| 50 | Journal P1 | ❌ Missing | Only `manifest.json` |

**Overall P0 coverage ≈ 55% (critical path shippable after Phase 0), P1 ≈ 25%.**

---

## 13. Prioritized Remediation Roadmap

### Week 0 — Blockers (must land before next tag)

- [ ] Fix `PROTECTED_PATHS "/"` `constants.rs:73` + separator guard `lib.rs:27`
- [ ] Fix `registry.rs:155` `EstimatedSize` + `registry.rs:160` `SystemComponent`
- [ ] Fix `services.rs:53` PID + `common.rs:21` `is_elevated` feature → unblock `clippy -D warnings`
- [ ] Fix `cli/main.rs:193,296` file writes + `utils.rs:159` registry parser
- [ ] Upgrade `h2` `Cargo.lock` + remove/feat-gate `reqwest`
- [ ] Change `selected_apps` to `HashSet<Uuid>` `app.rs:118`

### Week 1-2 — Stability

- [ ] Canonicalization + hardening `is_protected_path` tests, backup symlink handling `backup.rs:246`, atomic manifest
- [ ] `OnceLock<Regex>` `uninstaller.rs:389`, case-insensitive Msi, process-kill guard `uninstaller.rs:545`
- [ ] `sudo -n` timeout `linux.rs:320`, `trash` crate `macos.rs:397` + `utils.rs:108`
- [ ] `task_scheduler` single-object JSON `69`, `restore.rs:170` error masking
- [ ] Wire `verbose/json` flags `cli/main.rs:18`, unify fuzzy search

### Week 3-4 — Efficiency & Performance

- [ ] Trim `tokio` features, dedup `crossterm`/`hashbrown`/`getrandom`/`windows`, bump `cargo-llvm-cov`
- [ ] Parallel `ScannerManager::scan_all` `scanner.rs:97`, reuse `System::new_all`, `du -sk` `macos.rs:236`
- [ ] `scan_parallel` repair + `rayon` merge, `OnceLock` batch

### Week 5-8 — Feature Completion (shippable v0.2)

- [ ] Silent-flag mapping, registry orphan analyzer full, portable heuristic, preview/sort/filter, progress ETA, tree view
- [ ] Register `browser_extensions`/`windows_features`/`startup` scanners, service `ImagePath` orphan, shell extension
- [ ] Recycle `IFileOperation`, elevation `CheckTokenMembership`, journal SQLite + backup TTL

### Next Quarter — V2

- Remote uninstall, plugin WASM, CVE monitor `features.md:2.1`, container/WSL awareness, enterprise dashboard.

---

## 14. Appendix — Verification Evidence

### 14.1 `cargo audit` (2026-09-01)

```
Loaded 1233 security advisories
Scanning Cargo.lock (378 crate dependencies)
error: 1 vulnerability found!
 Crate: h2 Version: 0.4.15 Title: h2 unbounded empty DATA frames ID: RUSTSEC-2026-0258 Solution: Upgrade to >=0.4.16
warning: 4 allowed warnings
 number_prefix 0.4.0 unmaintained (RUSTSEC-2025-0119)
 paste 1.0.15 unmaintained (RUSTSEC-2024-0436)
 lru 0.12.5 unsound ×2 (RUSTSEC-2026-0002, RUSTSEC-2026-0253)
```

### 14.2 `cargo deny`

```
warning[license-not-encountered]: BSD-2-Clause, ISC, Unicode-DFS-2016, OpenSSL unmatched
warning[duplicate]: core-foundation 0.9.4 vs 0.10.1, crossterm 0.28.1 vs 0.29.0,
 getrandom 0.2.17 vs 0.4.3, hashbrown 0.14.5 vs 0.15.5 vs 0.17.1
 (truncated — via reqwest/hyper → hyper-tls → greek-core → greek-cli/tui)
```

### 14.3 `cargo clippy -D warnings`

```
error: unexpected cfg condition value: `greek-windows`
 --> crates\greek-platform\src\common.rs:21:38
 --> crates\greek-platform\src\common.rs:26:42
 = note: `-D unexpected-cfgs` implied by `-D warnings`
error: could not compile `greek-platform` (lib) due to 2 previous errors
```

### 14.4 Key Source Locations Referenced

```
crates/greek-common/src/constants.rs:58-93,106-123
crates/greek-common/src/lib.rs:26-37
crates/greek-common/src/models.rs:11-58,241-267
crates/greek-common/src/error.rs:1-173
crates/greek-common/src/traits.rs:1-161
crates/greek-core/src/scanner.rs:51-82,97-122,196-278,416-456
crates/greek-core/src/uninstaller.rs:84-120,185-259,295,389-531,545-565
crates/greek-core/src/leftover.rs:61-301
crates/greek-core/src/config.rs:14-118
crates/greek-core/src/backup.rs:55-278
crates/greek-core/src/app_service.rs:34,120-451
crates/greek-core/src/utils.rs:34-205
crates/greek-core/src/browser_extensions.rs:82-567
crates/greek-core/src/task_scheduler.rs:69-291
crates/greek-core/src/windows_features.rs:68-447
crates/greek-windows/src/registry.rs:46-398
crates/greek-windows/src/store.rs:29-312
crates/greek-windows/src/services.rs:42-204
crates/greek-windows/src/restore.rs:15-241
crates/greek-windows/src/wmi.rs:16-301
crates/greek-windows/src/elevation.rs:13
crates/greek-windows/src/recycle.rs:37
crates/greek-windows/src/icon.rs:17-401
crates/greek-windows/src/system_stats.rs:70-331
crates/greek-platform/src/common.rs:20-38
crates/greek-platform/src/linux.rs:28-404
crates/greek-platform/src/macos.rs:52-507
crates/greek-cli/src/main.rs:18-495
crates/greek-tui/src/app.rs:118-858
crates/greek-tui/src/ui.rs:123-1240
crates/greek-tui/src/theme.rs:164-202
crates/greek-tui/src/events.rs:1-92
deny.toml:23,25,26,30,45
audit.toml:1-28
.github/workflows/ci.yml:32-218
Cargo.toml:22,52-61
Cargo.lock:40,51,93,105
```

---

*Generated for `REEK-uninstaller` audit1 — for `audit2` consider `cargo geiger` (unsafe audit), `cargo tarpaulin` diff vs `llvm-cov`, and fuzzing `parse_command_string` + `ps_escape` with `cargo-fuzz`.*
