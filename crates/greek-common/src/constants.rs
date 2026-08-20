// Constants for REEK Ultimate Uninstaller

/// Default configuration file locations
pub const DEFAULT_CONFIG_DIR: &str = ".reek";
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const BACKUP_DIR_NAME: &str = "backups";
pub const LOG_DIR_NAME: &str = "logs";

/// Default timeout values
pub const DEFAULT_UNINSTALL_TIMEOUT_SECONDS: u64 = 300;
pub const DEFAULT_SCAN_TIMEOUT_SECONDS: u64 = 60;
pub const DEFAULT_PROCESS_KILL_TIMEOUT_SECONDS: u64 = 10;

/// Safety thresholds
pub const DEFAULT_CONFIDENCE_THRESHOLD: f32 = 0.7;
pub const MAX_BACKUP_SIZE_MB: u64 = 100;

/// Registry paths for Windows
#[cfg(target_os = "windows")]
pub const REGISTRY_UNINSTALL_PATH_32: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
#[cfg(target_os = "windows")]
pub const REGISTRY_UNINSTALL_PATH_64: &str =
    r"Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

/// Common application directories
pub const COMMON_APP_DIRS: &[&str] = &[
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "AppData/Local",
    "AppData/Roaming",
];

/// File size limits
pub const MAX_FILE_SIZE_SCAN_BYTES: u64 = 1_000_000_000; // 1GB
pub const MAX_TOTAL_SCAN_SIZE_BYTES: u64 = 10_000_000_000; // 10GB

/// UI constants
pub const DEFAULT_FPS: u32 = 30;
pub const MIN_FPS: u32 = 10;
pub const MAX_FPS: u32 = 60;

/// Version information
pub const APPLICATION_NAME: &str = "reek-uninstaller";
pub const APPLICATION_DISPLAY_NAME: &str = "REEK Ultimate Uninstaller";

/// Protected paths that should never be deleted.
///
/// Force-removing any of these paths could brick the operating system or
/// destroy user data. The list is intentionally broad: on Windows it covers
/// the Windows directory tree, Program Files roots, user-profile roots, and
/// critical system directories. On Unix it covers `/bin`, `/sbin`, `/lib`,
/// `/usr`, `/etc`, `/var`, `/home`, and the root mount.
///
/// `is_protected_path()` in `utils.rs` performs a **case-insensitive
/// prefix match** against this list, so `C:\Windows\System32\config` is
/// correctly blocked even though only `C:\Windows` is listed.
pub const PROTECTED_PATHS: &[&str] = &[
    // ── Windows ──────────────────────────────────────────────────────
    r"C:\Windows",
    r"C:\Windows\System32",
    r"C:\Windows\SysWOW64",
    r"C:\Windows\WinSxS",
    r"C:\Program Files\WindowsApps",
    r"C:\Program Files",
    r"C:\Program Files (x86)",
    r"C:\ProgramData",
    r"C:\Users",
    r"C:\Recovery",
    r"\SystemRoot",
    r"\Windows\System32\drivers",
    // ── Unix / macOS ─────────────────────────────────────────────────
    "/",
    "/bin",
    "/sbin",
    "/lib",
    "/lib64",
    "/usr",
    "/usr/bin",
    "/usr/sbin",
    "/usr/lib",
    "/usr/lib64",
    "/usr/local",
    "/etc",
    "/var",
    "/home",
    "/root",
    "/opt",
    "/System",
    "/Library",
    "/Applications",
    "/Users",
];

/// Protected Windows registry paths.
///
/// `delete_registry_key()` must refuse to touch any key whose full path
/// (including hive prefix) starts with one of these strings. The list is
/// intentionally broad — it covers critical OS hives, core Windows
/// configuration, and service definitions.
///
/// Matching is **case-insensitive prefix match** (same strategy as file
/// paths), so `HKLM\SYSTEM\CurrentControlSet\Services\CriticalService`
/// is blocked by the `HKLM\SYSTEM` entry.
#[cfg(target_os = "windows")]
pub const PROTECTED_REGISTRY_PATHS: &[&str] = &[
    r"HKLM\SYSTEM",
    r"HKLM\SOFTWARE\Microsoft\Windows",
    r"HKLM\SOFTWARE\Microsoft\Windows NT",
    r"HKLM\SOFTWARE\Microsoft\Cryptography",
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce",
    r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon",
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies",
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer",
    r"HKLM\SYSTEM\CurrentControlSet\Services",
    r"HKLM\SYSTEM\CurrentControlSet\Control",
    r"HKCU\SOFTWARE\Microsoft\Windows",
    r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
    r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce",
    r"HKLM\SAM",
    r"HKLM\SECURITY",
];

/// Placeholder for non-Windows builds so the constant is always in scope.
#[cfg(not(target_os = "windows"))]
pub const PROTECTED_REGISTRY_PATHS: &[&str] = &[];
