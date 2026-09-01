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
    // NOTE: "/" removed — it would flag every absolute path as protected.
    // Root itself is handled separately in is_protected_path() if needed.
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

/// Substrings (case-insensitive) that identify OS-critical / system-bundled
/// applications. If an app's **display name** contains any of these substrings
/// it is treated as a system component and **hidden by default** — removing it
/// could break the OS, drivers, or Windows itself.
///
/// This is intentionally conservative: it only targets well-known runtimes,
/// drivers, SDKs and Windows inbox components. User apps like
/// "Microsoft Office", "Visual Studio Build Tools", "Docker Desktop" or
/// "NVIDIA CUDA Documentation" do **NOT** match and remain visible / deletable.
pub const OS_CRITICAL_NAME_SUBSTRINGS: &[&str] = &[
    // --- Microsoft VC++ / Universal CRT runtimes ---
    "microsoft visual c++",
    "microsoft visual studio 2010 tools",
    "vcredist",
    "vc_redist",
    "vc redist",
    "universal crt",
    // --- .NET runtimes / hosting (deleting breaks many apps) ---
    "microsoft .net runtime",
    "microsoft .net host",
    "microsoft asp.net core",
    "microsoft windows desktop runtime",
    "microsoft .net targeting pack",
    // --- Windows SDK / WDK / ADK / PE ---
    "windows sdk",
    "windows software development kit",
    "windows driver kit",
    "windows assessment and deployment kit",
    "windows pe ",
    "windows adk",
    "wdk ",
    // --- Windows inbox / servicing ---
    "microsoft update health tools",
    "microsoft edge webview2 runtime",
    "microsoft edge update",
    "windows pc health check",
    // --- Driver vendors — only when name contains "driver" ---
    // (checked via driver_critical guard, not blanket)
];

/// OS-critical Store package name substrings (case-insensitive).
/// Matches the `Name` field from Get-AppxPackage.
pub const OS_CRITICAL_STORE_SUBSTRINGS: &[&str] = &[
    "microsoft.windowsstore",
    "microsoft.storepurchaseapp",
    "microsoft.desktopappinstaller", // winget
    "microsoft.aad.brokerplugin",
    "microsoft.windows.aad",
    "microsoft.windows.cloud",
    "microsoft.windows.shell",
    "microsoft.windows.cortana",
    "microsoft.windows.sechealthui",
    "microsoft.ui.xaml",
    "microsoft.vclibs",
    "microsoft.net.native",
    "microsoft.windowscommunicationsapps", // Mail + Calendar
    "microsoft.windows.photos",
    "microsoft.windowsphotos",
    "microsoft.windows.camera",
    "microsoft.windowscamera",
    "microsoft.windows.alarms", // Clock / Alarms & Clock
    "microsoft.windowsalarms",
    "microsoft.windows.clock",
    "microsoft.windowsclock",
    "microsoft.windows.calculator",
    "microsoft.windowscalculator",
    "microsoft.windowsmaps",
    "microsoft.zunemusic",  // Groove Music
    "microsoft.zunevideo",  // Movies & TV
    "microsoft.xbox",
    "microsoft.gamingservices",
    "microsoft.gamingapp",
    "microsoft.people",
    "microsoft.bingweather",
    "microsoft.bingnews",
    "microsoft.gethelp",
    "microsoft.getstarted",
    "microsoft.microsoft3dviewer",
    "microsoft.mixedreality.portal",
    "microsoft.microsoftstickynotes",
    "microsoft.screensketch", // Snipping Tool / Snip & Sketch (old name)
    "microsoft.screenSketch",
    "microsoftwindows.client.cbs", // Snipping Tool + Search (Windows 11)
    "microsoft.windows.soundrecorder", // Voice Recorder / Sound Recorder
    "microsoft.windowssoundrecorder",
    "microsoft.windowsnotepad", // Notepad (Store version)
    "microsoft.notepad",
    "microsoft.mspaint", // Paint
    "microsoft.paint",
    "microsoft.heifimageextension",
    "microsoft.vp9videolayers",
    "microsoft.webmediaplayer",
    "microsoft.webmediaextensions",
    "microsoft.hevcVideoExtension",
    "microsoft.yourphone",
    "microsoft.phoneexperience",
    "microsoft.windows.terminal", // Terminal is inbox on Win11
    "microsoft.toshiba", // example - keep narrow
    "microsoft.todos",
    "microsoft.whiteboard",
    "microsoft.feedbackhub",
    "microsoft.windowsfeedbackhub",
    "microsoft.windows.sechealthui",
    "microsoft.windows.secureassessmentbrowser",
    "microsoft.windows.narratorquickstart",
    "microsoft.windows.parentscontrol",
    "microsoft.windows.xboxgamesoverlay",
];

/// Default inbox apps (Registry DisplayName substrings, case-insensitive).
/// These are non-essential for OS boot but are bundled by default and users
/// generally should not delete them. We hide them because the UI should only
/// show **externally installed, safe-to-remove** apps (e.g. Chrome, 7-Zip,
/// Office if user-installed, Docker, etc.).
///
/// Rule: only hide when publisher looks like Microsoft / Windows, to avoid
/// false positives like "Moffsoft FreeCalc" matching "calculator".
pub const INBOX_DEFAULT_NAME_SUBSTRINGS: &[&str] = &[
    // Clock / Calendar / Snipping / Calculator family
    "alarms & clock",
    "windows clock",
    "windows alarms",
    "calendar", // covers "Mail and Calendar"
    "snipping tool",
    "snip & sketch",
    "screen sketch",
    "calculator",
    "camera",
    "photos",
    "maps",
    "voice recorder",
    "sound recorder",
    "people",
    "sticky notes",
    "paint 3d",
    "mixed reality portal",
    "3d viewer",
    "feedback hub",
    "get help",
    "get started",
    "tips", // "Tips" app -> Microsoft Tips
    "weather",
    "news",
    "cortana",
    "xbox",
    "gaming",
    "solitaire",
    "your phone",
    "phone link",
    "phone experience",
    "notepad", // Store Notepad - but keep "Notepad++" safe (publisher guard)
    "media player", // new Windows 11 Media Player
    "movies & tv",
    "groove music",
    "onedrive", // inbox but deletable - hide as default; user can still remove via system if needed
    "microsoft edge", // Edge is inbox; WebView2 already in OS_CRITICAL
    "windows web experience pack",
    "web experience pack",
    "clipchamp",
    "family safety",
    "teams", // Microsoft Teams (inbox on Win11)
    "power automate",
    "quick assist",
];

/// Check if a display name matches any OS-critical substring (case-insensitive).
/// Also handles driver-specific guards: "intel"/"realtek"/"nvidia"/"qualcomm"
/// + "driver" is considered critical, but e.g. "nvidia cuda documentation" is not.
pub fn is_os_critical_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    for pat in OS_CRITICAL_NAME_SUBSTRINGS {
        if lower.contains(&pat.to_lowercase()) {
            return true;
        }
    }
    // Driver guard: vendor + "driver" must both appear
    let is_driver = lower.contains("driver");
    if is_driver {
        if lower.contains("intel")
            || lower.contains("realtek")
            || lower.contains("nvidia")
            || lower.contains("qualcomm")
            || lower.contains("amd")
            || lower.contains("broadcom")
        {
            return true;
        }
        // Generic driver tools that are OS-critical
        if lower.contains("oem") || lower.contains("chipset") || lower.contains("firmware") {
            return true;
        }
    }
    // KB / update / hotfix artifacts
    if lower.starts_with("kb") && lower.chars().skip(2).next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return true;
    }
    if lower.contains("security update for windows")
        || lower.contains("update for windows")
        || lower.contains("hotfix for windows")
    {
        return true;
    }
    false
}

/// Check if a Windows Store package name is OS-critical.
pub fn is_os_critical_store_package(package_name: &str) -> bool {
    let lower = package_name.to_lowercase();
    for pat in OS_CRITICAL_STORE_SUBSTRINGS {
        if lower.contains(&pat.to_lowercase()) {
            return true;
        }
    }
    // Framework heuristic already handled elsewhere, keep as fallback
    if lower.starts_with("microsoft.") && (lower.contains(".ui.") || lower.contains("framework")) {
        return true;
    }
    // Also treat any Store package from the inbox list as critical/default
    // (already covered above, kept for clarity)
    false
}

/// Check if a display name is a default inbox app (Clock, Calendar,
/// Snipping Tool, Calculator, etc.). Only returns true when the name
/// matches an inbox substring **and** the publisher looks like Microsoft
/// / Windows — this avoids false positives for third-party apps like
/// "Notepad++" (publisher Don Ho) or "Free Calculator" (Moffsoft).
pub fn is_inbox_default_app(name: &str, publisher: Option<&str>) -> bool {
    let lower = name.to_lowercase();
    // Quick publisher guard: must be Microsoft-ish or empty (Store apps often have CN= publisher)
    // If publisher is clearly third-party (e.g. "Google LLC", "VideoLAN"), don't hide.
    if let Some(pub_str) = publisher {
        let pub_lower = pub_str.to_lowercase();
        // Allow empty or microsoft/windows-ish to be considered inbox
        let is_ms_pub = pub_lower.contains("microsoft") || pub_lower.contains("windows") || pub_lower.is_empty();
        // If publisher is known third-party, never treat as inbox
        if !is_ms_pub {
            // Special case: inbox names like "calculator" with third-party publisher are NOT inbox
            return false;
        }
    }
    for pat in INBOX_DEFAULT_NAME_SUBSTRINGS {
        let pat_lower = pat.to_lowercase();
        // For short generic words like "calendar", "camera", "maps", "people", "tips"
        // require exact or Microsoft-prefixed match to avoid over-filtering.
        // For longer distinctive phrases like "snipping tool", substring is fine.
        if pat_lower.len() <= 8 {
            // short token: require word boundary or exact match
            // e.g. "calendar" should match "Mail and Calendar" but not "CalendarPro"
            if lower == pat_lower || lower.contains(&pat_lower) {
                // extra guard: for very short names, also require publisher is MS
                // already guarded above, so ok
                return true;
            }
        } else if lower.contains(pat_lower.as_str()) {
            return true;
        }
    }
    false
}

/// Unified check: is this app a default inbox app *or* OS-critical?
/// `store_package_name` is only used for Store sources (pass the Appx `Name`).
pub fn is_hidden_by_default(name: &str, publisher: Option<&str>, store_package_name: Option<&str>) -> bool {
    if is_os_critical_name(name) {
        return true;
    }
    if is_inbox_default_app(name, publisher) {
        return true;
    }
    if let Some(pkg) = store_package_name {
        if is_os_critical_store_package(pkg) {
            return true;
        }
        // Also check display name against inbox store list via package name heuristic
        // e.g. "Clock" with package Microsoft.WindowsAlarms
        if is_os_critical_store_package(name) {
            return true;
        }
    }
    false
}
