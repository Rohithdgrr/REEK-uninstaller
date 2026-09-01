// Structured logging for REEK - JSON file sink with rotation + stdout layer.
// Replaces println! with tracing; secrets are redacted by callers via sanitize_output.

use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Returns platform log directory: <data_dir>/logs (e.g. %APPDATA%/reek-uninstaller/logs).
pub fn log_dir() -> PathBuf {
    _utils_get_dirs()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(crate::LOG_DIR_NAME)
}

/// Best-effort data dir lookup without depending on greek_core.
fn _utils_get_dirs() -> Result<PathBuf, String> {
    let proj = directories::ProjectDirs::from("com", "reek", "reek-uninstaller")
        .ok_or_else(|| "cannot resolve project dirs".to_string())?;
    Ok(proj.data_dir().to_path_buf())
}

fn ensure_log_dir() -> PathBuf {
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    dir
}

/// Initialise global tracing subscriber.
///
/// - `verbose`: if true, default level DEBUG else INFO (overridden by RUST_LOG).
/// - Writes **JSON** to daily-rotated file `logs/reek.log.YYYY-MM-DD` (non-blocking).
/// - Writes pretty human-readable logs to stdout.
/// - Redaction of secrets is done at call-sites (sanitize_output/redact_secrets).
///
/// Returns a `WorkerGuard` that must be held for the lifetime of the process;
/// dropping it flushes the file buffer. Callers should store it (e.g. `_guard`).
pub fn init_logging(verbose: bool) -> Result<WorkerGuard, String> {
    let dir = ensure_log_dir();

    // Non-blocking file appender: daily rotation, keep files 14 days via manual prune elsewhere.
    let file_appender = tracing_appender::rolling::daily(&dir, "reek.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let filter = if std::env::var("RUST_LOG").is_ok() {
        EnvFilter::from_default_env()
    } else if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    // JSON layer for file sink
    let file_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(file_writer)
        .with_ansi(false);

    // Human-readable layer for stderr (pretty, with colors when tty)
    // Uses stderr so CLI stdout remains clean for `reek list --format json/csv` piping.
    let stdout_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(atty::is(atty::Stream::Stderr))
        .with_target(true);

    // Try to set global subscriber; if already set (e.g. in tests) just return guard.
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stdout_layer);

    match tracing::subscriber::set_global_default(subscriber) {
        Ok(_) => {
            tracing::info!(
                log_dir = %dir.display(),
                verbose = verbose,
                "tracing initialized (JSON file + stdout)"
            );
            Ok(guard)
        }
        Err(e) => {
            // Already initialized (common in tests) – still return guard so file sink lives.
            eprintln!("tracing already initialized: {}", e);
            Ok(guard)
        }
    }
}

/// Prune log files older than `retention_days` (best-effort, logs dir only).
pub fn prune_old_logs(retention_days: u64) {
    let dir = log_dir();
    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(retention_days * 24 * 3600);
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                if mtime < cutoff {
                    let _ = std::fs::remove_file(entry.path());
                    tracing::info!("pruned old log {}", entry.path().display());
                }
            }
        }
    }
    // Enforce permissions on remaining files (0o600 on unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let _ = std::fs::set_permissions(e.path(), std::fs::Permissions::from_mode(0o600));
            }
        }
    }
}
