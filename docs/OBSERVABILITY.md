# Observability

How to diagnose REEK in the field when uninstall fails at 3 AM.

## Logs

- **Location**: `<data_dir>/logs/reek.log.YYYY-MM-DD` (daily rotation)
  - Windows: `%APPDATA%\reek\reek-uninstaller\logs\`
  - Linux: `~/.local/share/reek-uninstaller/logs/`
  - macOS: `~/Library/Application Support/com.reek.reek-uninstaller/logs/`
- **Format**: JSON per line (`tracing-subscriber` `json` + file layer), `INFO` by default.
- **Stdout**: human-readable `fmt` to `stderr` (so `reek list --format json | jq` pipes cleanly).
- **Control**: `RUST_LOG=debug reek list` or `reek --verbose` (CLI) overrides level.
- **Redaction**: `sanitize_output` caps stdout/stderr at 8KB, `redact_secrets` masks `/token=`, `password=`, `key=`.

## Retention

- File appender: `tracing_appender::rolling::daily` (one file per day).
- Prune: `greek_common::logging::prune_old_logs(14)` removes files older than 14 days on start (both `reek` and `reek-tui`).
- Permissions: `0o700` on log dir, `0o600` on log files; temp state `.reek_state.json` also `0o600` on Unix.

## Metrics (opt-in)

No network telemetry by default. When enabled via config `telemetry.enabled = true` (future), metrics use `tracing` counters:

- `scan_apps_total`, `uninstall_success_total`, `uninstall_failure_total`, `delete_protected_blocked_total`
- Exposed via `tracing` events; consumers can attach an OpenTelemetry layer.

## Crash reporting

- `color-eyre` + panic hook restores terminal (TUI) and logs panic payload via `tracing::error!`.
- Opt-in Sentry layer can be added: set `SENTRY_DSN` env var then `sentry-tracing` layer (not enabled by default to avoid exfil).

## Ask for a bug report

When user opens a ticket, ask for:

1. `reek --version` + OS
2. `reek.log.YYYY-MM-DD` tail (redacted)
3. Steps + `RUST_LOG=debug` reproduction log
