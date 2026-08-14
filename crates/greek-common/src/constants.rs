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

/// Protected paths that should never be deleted
pub const PROTECTED_PATHS: &[&str] = &[
    r"C:\Windows",
    r"C:\Windows\System32",
    r"C:\Program Files\WindowsApps",
    r"C:\Program Files",
    r"C:\Program Files (x86)",
    r"C:\Windows\SysWOW64",
    r"\SystemRoot",
    r"\Windows\System32\drivers",
    r"/System",
    r"/usr",
    r"/bin",
    r"/sbin",
    r"/lib",
    r"/lib64",
    r"/usr/bin",
    r"/usr/sbin",
    r"/usr/lib",
    r"/usr/lib64",
];
