# Security Architecture

This document explains *how* REEK keeps the system safe while doing destructive
work. For the threat model, supported-version policy, and how to report a
vulnerability, see the repository root [SECURITY.md](../SECURITY.md).

---

## The core problem

REEK's job is: remove a program's files, registry keys, services and processes,
and clean up leftovers — all of it destructive and all of it guided by data
that predominantly comes from **the operating system and from the app itself**
(scanner output, `UninstallString`, registry entries, install locations). That
data is **attacker-influenced**: a malicious app can write whatever it wants to
its own registry keys before REEK reads them.

The architecture mitigates this in layers.

## Layer 1 — Segmentation of responsibilities

```
greek-common    shared types: SafetyLevel, UninstallOptions, InstalledApp, PROTECTED_PATHS
greek-core      business logic + strategies (parse, decide, execute)
greek-windows   platform capabilities (registry, services, restore, WMI)
greek-platform  Linux/macOS scanners (command-based, not native bindings)
greek-cli       headless front-end
greek-tui       terminal front-end
```

Safety-relevant decisions (`SafetyLevel`, `PROTECTED_PATHS`, parsing, strategy
selection) live in `greek-core`/`greek-common`. Platform code (`greek-windows`)
only *executes* decisions; it cannot decide what to delete by itself. This keeps
"what is dangerous" in one auditable place.

## Layer 2 — Never trust the uninstall string

`UninstallString` comes from the registry. A hostile `UninstallString` would
normally be the classic "the registry lets the app run anything" attack. REEK
does not fix that (the app can run its own uninstaller by design), but it
bounds the damage:

- **Quote-aware tokenization** (`parse_command_string`): splits
  `"C:\Program Files\App\uninstall.exe" /S` into `program` + `args` correctly,
  with a `shell_words` fallback. See `uninstaller.rs`.
- **No shell.** Commands are launched with `Command::new(program).args(args)`.
  Shell metacharacters (`&`, `|`, `;`, `>`) remain literal arguments and cannot
  chain extra commands.
- **Timeout.** Every uninstall runs inside
  `tokio::time::timeout(DEFAULT_UNINSTALL_TIMEOUT_SECONDS)`, bounding how long a
  malicious or broken uninstaller can hold the system.

## Layer 3 — Never delete protected paths

`PROTECTED_PATHS` (constants.rs) enumerates:

- Windows: `C:\Windows`, `System32`, `Program Files`, `WindowsApps`, ...
- Unix: `/usr`, `/bin`, `/sbin`, `/lib`, `/System`, ...

`ForceRemoveStrategy` only deletes the recorded `install_location` for the app,
and `is_protected_path()` gates deletion decisions. Deleting a path under a
protected root is not possible through the normal flow.

## Layer 4 — System Restore as a safety net

Before uninstalling (when `create_restore_point` is on, which is the default),
`GreekAppService::create_restore_point()` invokes the Windows System Restore API
via the PowerShell `Checkpoint-Computer` command. If creation fails (restore
disabled, already in progress), REEK **logs a warning and continues** — running
without a restore point is preferable to blocking an uninstall, and the failure
is visible in the log output.

> Note: the restore-point command is a shell-out to PowerShell with the
> description single-quote-escaped. The description originates from the app
> name (scanner data); escaping prevents PowerShell injection from that field.

## Layer 5 — Conservative leftover removal

Leftovers are never assumed safe:

- Every `LeftoverArtifact` has a `SafetyLevel` and a `confidence` score.
- Only `SafetyLevel::Safe` artifacts pass `is_safe_to_delete()` and may be
  removed without explicit confirmation.
- The confidence threshold default (`0.7`) is configurable but validated to be
  in `[0.0, 1.0]` by `ConfigManager::validate_config`.

## Layer 6 — Scoped process termination

`kill_processes_by_path` enumerates processes via `sysinfo` and kills only those
whose executable **path begins with** the target's install directory. This
avoids the "kill by name" foot-gun where two vendors ship identically-named
processes.

## Layer 7 — Build-time security (defense in the pipeline)

- Committed `Cargo.lock` → deterministic, audited dependency set.
- `cargo-audit` → blocks known-vulnerable crates.
- `cargo-deny` → blocks unlicensed/unapproved deps, wildcard version
  requirements, unknown registries, unmaintained/yanked workspace deps.
- Pinned CI action SHAs + `permissions: contents: read` → a compromised
  third-party action cannot steal write tokens.

See [CI_CD.md](CI_CD.md) and [SECURITY.md](../SECURITY.md).

## Known gaps (documented, not yet implemented)

| Gap | Impact | Mitigation status |
|-----|--------|-------------------|
| REEK executes whatever `UninstallString` points to | Malicious app can run its own (already-signed) uninstaller — user sees it happen | Inherent to uninstallers; warn user before running unknown apps |
| `move_to_recycle_bin` currently deletes directly on all platforms (recycle bin not implemented) | Removed files are not recoverable | Consider implementing recycle-bin/trash integration |
| Uninstaller runs with user's privileges by default | Cannot touch admin-only paths unless REEK is elevated | Run REEK elevated on Windows for full cleanup |
| Restore point relies on PowerShell `Checkpoint-Computer` | Requires System Restore enabled + privileges | Failure is non-fatal (logged) |

## Security review checklist

Before merging code that touches these areas, re-audit:

1. New code paths that receive a `PathBuf` from disk or registry input.
2. Any new `Command::new` / shell invocation — confirm no user/source-controlled
   string reaches a shell unescaped.
3. Changes to `PROTECTED_PATHS`, `SafetyLevel`, or deletion logic in
   `ForceRemoveStrategy`.
4. New dependencies: acceptable license, maintained, pinned, audited.