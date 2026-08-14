# Security Policy

REEK Ultimate Uninstaller is a system-modifying tool. It removes files, deletes
registry keys, kills processes, stops and deletes services, and can create
system restore points. This document describes the threat model, the security
controls in place, and how to report vulnerabilities.

**Supported versions:** `0.1.x` (current development series). Security fixes are
backported to the latest released minor version.

---

## Reporting a Vulnerability

If you discover a security issue in REEK, **do not open a public GitHub issue**.
Report it privately.

- **Email:** `security@greek.io` (PGP key available on request)
- **Alternative:** GitHub private vulnerability reporting
  (Repository → Security → Report a vulnerability)

Please include, if possible:

1. The affected version and platform (Windows / Linux / macOS).
2. A minimal reproduction (steps, commands, or a crafted registry / uninstall
   string).
3. The impact you believe the issue has (e.g. arbitrary code execution during
   uninstall, privilege escalation, path traversal, deletion of protected paths).

We will acknowledge receipt within **48 hours** and aim to triage within
**7 days**. You will be credited in the advisory unless you request otherwise.
Coordinated disclosure: please allow 90 days before public disclosure.

---

## Threat Model

REEK runs **with the privileges of the invoking user**, and on Windows it may
operate against Program Files / HKLM keys the user can already modify when
elevated. The primary attackers in scope are:

| Attacker | Capability | Example |
|----------|------------|---------|
| Malicious installed app | Can place files and registry entries on disk during its own install | Subverts REEK by writing a malicious `UninstallString`, service, or `.exe` |
| Malicious package / store app | Same, via an app the user installed | A store package registers `ExePath` that points to hostile code |
| Local non-privileged user | Can run REEK, but is not elevated | Exploits REEK to delete protected/system paths they could not otherwise touch |
| Supply-chain | Can tamper with dependencies or CI | Compromised crate or CI action injects malicious code |

### Assets to protect

- **User data** (`C:\Users\...`, `$HOME`): never delete unless explicitly
  confirmed leftover data for the app being removed.
- **System integrity** (`C:\Windows`, `/System`, `/usr`, ...): never delete —
  enforced by `PROTECTED_PATHS`.
- **Process integrity of REEK itself**: REEK must not blindly execute commands
  from attacker-controlled strings.
- **Build reproducibility**: pinned `Cargo.lock`, pinned CI action SHAs, and
  `cargo-audit` + `cargo-deny` gates.

---

## Security Controls

### 1. Command execution is a *controlled* action

The biggest risk is `UninstallString` — an attacker-controlled command line REEK
executes as the user.

- Uninstall commands are parsed with a quote-aware tokenizer
  (`StandardUninstallStrategy::parse_command_string`) instead of naive splitting,
  and fall back to `shell_words` for correctness. See
  `crates/greek-core/src/uninstaller.rs:211`.
- The parsed command runs **without a shell** via `std::process::Command` —
  there is no `cmd.exe /C` interpolation step, so piped `/` `|`, `&` sequences
  in an uninstall string are **arguments, not operators**.
- `Command::output()` is wrapped in a `tokio::time::timeout` so a hung uninstaller
  cannot block REEK indefinitely (`DEFAULT_UNINSTALL_TIMEOUT_SECONDS = 300`).
- MSI product codes are extracted with a strict UUID regex
  (`MsiUninstallStrategy::extract_product_code`).

**Gap / future work:** REEK executes the uninstaller supplied by the registry. A
malicious application can legitimately trigger *its own* binaries; REEK cannot
distinguish this from a normal uninstall. Do not run REEK against applications
you do not trust.

### 2. Protected paths are never deleted

`PROTECTED_PATHS` in `crates/greek-common/src/constants.rs` lists system paths
(`C:\Windows`, `C:\Program Files`, `/usr`, `/System`, ...). `is_protected_path`
is available for deletion gating, and `ForceRemoveStrategy` deletes only the
recorded `install_location`.

### 3. Restore points and registry backups

- `UninstallOptions::standard()` enables `create_restore_point: true`. Before an
  uninstall, `GreekAppService::create_restore_point` creates a Windows System
  Restore point (`crates/greek-windows/src/restore.rs`).
- Restore-point creation failures are logged and do **not** fail the uninstall
  (the uninstall may still succeed without a restore point).
- `ForceRemoveStrategy` deletes registry keys only via the validated
  `delete_registry_key` path parser (hive prefix required; rejects ambiguous
  paths).

### 4. Process killing is path-scoped

`ForceRemoveStrategy::kill_processes_by_path` kills processes whose executable
path **starts with** the app's install directory — it does not kill by name or
PID, reducing the blast radius to the app being removed.

### 5. Leftover detection is conservative

`LeftoverArtifact` carries a `SafetyLevel` (`Safe` → `Caution` → `Dangerous` →
`Critical`) and a confidence score. Only `Safe` artifacts are auto-deletable
(`is_safe_to_delete`); everything else requires confirmation. The default
confidence threshold (`DEFAULT_CONFIDENCE_THRESHOLD = 0.7`) must be exceeded.

### 6. Supply-chain and build integrity

- **Lockfile:** `Cargo.lock` is committed (application workspace) so every build
  resolves the exact dependency versions that were tested.
- **CI action pinning:** every GitHub Action in
  `.github/workflows/ci.yml` is pinned to a full commit SHA (immutable),
  with the tag as a comment.
- **Least-privilege CI:** all workflow jobs declare `permissions: contents: read`
  — no write tokens are exposed to untrusted steps.
- **`cargo-audit`:** fails on any vulnerability advisory (RustSec database).
- **`cargo-deny`:** denies unknown/unapproved licenses, wildcard dependencies,
  unknown registries, and unmaintained / yanked workspace dependencies.
- **Dependency versions** are pinned with `=`-style ranges via `Cargo.lock`
  rather than `*` requirements.

### 7. MSRV enforcement

The workspace declares `rust-version = "1.78.0"`; CI runs a dedicated MSRV job
so code never silently depends on newer compiler features.

---

## In-scope areas for review

Reviewers should focus on:

- `crates/greek-core/src/uninstaller.rs` — command parsing & execution.
- `crates/greek-core/src/app_service.rs` — restore point flow, batch handling.
- `crates/greek-core/src/utils.rs` — `delete_registry_key`, path helpers.
- `crates/greek-windows/src/*` — registry, services, restore, WMI.
- Any code that takes `PathBuf` from disk and feeds it to the filesystem.

## Out of scope

- Apps bundled with the OS (malicious OS components are not REEK's threat).
- Physical access / local kernel compromise.

---

## Disclosure process

1. Reporter sends details; maintainers acknowledge.
2. Maintainers confirm, triage severity (CVSS), and fix.
3. A fix + regression test land with the advisory mentioned in `CHANGELOG.md`.
4. After 90 days (or earlier if a public fix exists), the advisory is published.