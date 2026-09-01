// Core data models for REEK Ultimate Uninstaller

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Clean an X.500 / Windows publisher string (e.g. "CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond")
/// into a readable company name. Also strips "Company (Product)" style suffixes.
pub fn clean_publisher_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // X.500 format: prefer CN=, fall back to O= (must be at start or after a comma)
    let mut has_cn = false;
    let mut o_start = None;
    let mut best: Option<String> = None;
    for (i, part) in trimmed.split(',').enumerate() {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("CN=") {
            has_cn = true;
            best = Some(rest.trim().to_string());
            break;
        }
        if (i == 0 || part.starts_with("O=")) && o_start.is_none() {
            if let Some(rest) = part.strip_prefix("O=") {
                o_start = Some(rest.trim().to_string());
            }
        }
    }
    if has_cn {
        if let Some(b) = &best {
            if !b.is_empty() {
                return b.clone();
            }
        }
    }
    if let Some(o) = o_start {
        if !o.is_empty() {
            return o;
        }
    }

    // "Company (Product)" or "Company - Product" → "Company"
    for sep in [" (", " - ", ", "] {
        if let Some(idx) = trimmed.find(sep) {
            let head = trimmed[..idx].trim();
            if head.len() >= 3 {
                return head.to_string();
            }
        }
    }

    trimmed.to_string()
}

/// Represents a discovered installed application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    pub id: Uuid,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub install_date: Option<NaiveDate>,
    pub install_location: Option<PathBuf>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
    pub modify_string: Option<String>,
    pub size_bytes: Option<u64>,
    pub source: InstallSource,
    pub icon_path: Option<PathBuf>,
    pub is_system_component: bool,
    pub estimated_leftover_size: Option<u64>,
    pub registry_keys: Vec<RegistryKey>,
    pub metadata: HashMap<String, String>,
}

impl InstalledApp {
    pub fn new(name: String, source: InstallSource) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            publisher: None,
            version: None,
            install_date: None,
            install_location: None,
            uninstall_string: None,
            quiet_uninstall_string: None,
            modify_string: None,
            size_bytes: None,
            source,
            icon_path: None,
            is_system_component: false,
            estimated_leftover_size: None,
            registry_keys: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn display_name(&self) -> String {
        if let Some(ref version) = self.version {
            format!("{} {}", self.name, version)
        } else {
            self.name.clone()
        }
    }

    /// Returns a human-readable size string, or `None` if the size is unknown.
    pub fn display_size(&self) -> Option<String> {
        self.size_bytes
            .map(|size| humansize::format_size(size, humansize::BINARY))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallSource {
    Registry {
        hive: RegistryHive,
        key_path: String,
    },
    WindowsStore {
        package_family_name: String,
        package_full_name: String,
    },
    Portable {
        detected_path: PathBuf,
        confidence: f32,
    },
    BrowserExtension {
        browser: BrowserType,
        extension_id: String,
    },
    WindowsFeature {
        feature_name: String,
    },
    PackageManager {
        manager: PackageManager,
        package_id: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistryHive {
    Hklm,
    Hkcu,
}

impl RegistryHive {
    pub fn as_str(&self) -> &'static str {
        match self {
            RegistryHive::Hklm => "HKLM",
            RegistryHive::Hkcu => "HKCU",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserType {
    Chrome,
    Firefox,
    Edge,
    Safari,
    Opera,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PackageManager {
    Winget,
    Scoop,
    Chocolatey,
    Apt,
    Dpkg,
    Rpm,
    Pacman,
    Homebrew,
    MacPorts,
    Flatpak,
    Snap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryKey {
    pub path: String,
    pub hive: RegistryHive,
    pub values: HashMap<String, RegistryValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryValue {
    pub value_type: RegistryValueType,
    pub data: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistryValueType {
    Sz,
    ExpandSz,
    Binary,
    Dword,
    DwordBigEndian,
    Link,
    MultiSz,
    Qword,
    None,
}

/// Represents a leftover artifact detected after uninstallation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeftoverArtifact {
    pub id: Uuid,
    pub app_id: Option<Uuid>,
    pub artifact_type: ArtifactType,
    pub path: PathBuf,
    pub size_bytes: Option<u64>,
    pub confidence: f32,
    pub safety_level: SafetyLevel,
    pub description: String,
    pub created_date: Option<NaiveDate>,
    pub last_modified: Option<NaiveDate>,
}

impl LeftoverArtifact {
    pub fn new(artifact_type: ArtifactType, path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            app_id: None,
            artifact_type,
            path,
            size_bytes: None,
            confidence: 0.5,
            safety_level: SafetyLevel::Caution,
            description: String::new(),
            created_date: None,
            last_modified: None,
        }
    }

    pub fn is_safe_to_delete(&self) -> bool {
        matches!(self.safety_level, SafetyLevel::Safe)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ArtifactType {
    Directory,
    File,
    RegistryKey,
    RegistryValue,
    Service,
    ScheduledTask,
    ShellExtension,
    Driver,
    Shortcut,
    Font,
    TempFile,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SafetyLevel {
    Safe = 0,
    Caution = 1,
    Dangerous = 2,
    Critical = 3,
}

impl SafetyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyLevel::Safe => "Safe",
            SafetyLevel::Caution => "Caution",
            SafetyLevel::Dangerous => "Dangerous",
            SafetyLevel::Critical => "Critical",
        }
    }
}

/// Options for uninstallation operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UninstallOptions {
    pub silent: bool,
    pub force: bool,
    pub timeout_seconds: Option<u64>,
    pub create_restore_point: bool,
    pub backup_registry: bool,
    pub move_to_recycle_bin: bool,
    pub kill_processes: bool,
    pub delete_services: bool,
    pub delete_tasks: bool,
    pub delete_leftovers: bool,
}

impl UninstallOptions {
    pub fn standard() -> Self {
        Self {
            silent: false,
            force: false,
            timeout_seconds: Some(300),
            create_restore_point: true,
            backup_registry: true,
            move_to_recycle_bin: false,
            kill_processes: true,
            delete_services: true,
            delete_tasks: true,
            delete_leftovers: false,
        }
    }

    pub fn silent() -> Self {
        let mut opts = Self::standard();
        opts.silent = true;
        opts
    }

    pub fn force() -> Self {
        let mut opts = Self::standard();
        opts.force = true;
        opts.delete_leftovers = true;
        opts
    }
}

/// Result of an uninstallation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallResult {
    pub app_id: Uuid,
    pub success: bool,
    pub strategy_used: String,
    pub exit_code: Option<i32>,
    pub duration: std::time::Duration,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub files_deleted: Vec<PathBuf>,
    pub registry_keys_deleted: Vec<String>,
    pub services_stopped: Vec<String>,
    pub errors: Vec<String>,
    pub restore_point_id: Option<String>,
    pub backup_id: Option<Uuid>,
}

impl Default for UninstallResult {
    fn default() -> Self {
        Self {
            app_id: Uuid::new_v4(),
            success: false,
            strategy_used: String::new(),
            exit_code: None,
            duration: std::time::Duration::ZERO,
            stdout: None,
            stderr: None,
            files_deleted: Vec::new(),
            registry_keys_deleted: Vec::new(),
            services_stopped: Vec::new(),
            errors: Vec::new(),
            restore_point_id: None,
            backup_id: None,
        }
    }
}

/// Batch operation queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchQueue {
    pub items: Vec<BatchItem>,
    pub options: UninstallOptions,
}

impl BatchQueue {
    pub fn new(options: UninstallOptions) -> Self {
        Self {
            items: Vec::new(),
            options,
        }
    }

    pub fn add_item(&mut self, app: InstalledApp) {
        self.items.push(BatchItem {
            app,
            status: BatchStatus::Queued,
            result: None,
        });
    }

    pub fn pending_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == BatchStatus::Queued)
            .count()
    }

    pub fn completed_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == BatchStatus::Completed)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == BatchStatus::Failed)
            .count()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    pub app: InstalledApp,
    pub status: BatchStatus,
    pub result: Option<UninstallResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BatchStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GreekConfig {
    pub ui: UiConfig,
    pub scanner: ScannerConfig,
    pub uninstall: UninstallConfig,
    pub leftover: LeftoverConfig,
    pub backup: BackupConfig,
    pub safety: SafetyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub show_icons: bool,
    pub confirm_destructive: bool,
    pub animation_fps: u32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "greek-blue".to_string(),
            show_icons: true,
            confirm_destructive: true,
            animation_fps: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    pub scan_portable_dirs: Vec<String>,
    pub scan_browser_extensions: bool,
    pub scan_windows_features: bool,
    pub scan_startup_items: bool,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            scan_portable_dirs: Vec::new(),
            scan_browser_extensions: true,
            scan_windows_features: false,
            scan_startup_items: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallConfig {
    pub default_timeout_seconds: u64,
    pub auto_detect_silent: bool,
    pub kill_processes_before_uninstall: bool,
    pub create_restore_point: bool,
}

impl Default for UninstallConfig {
    fn default() -> Self {
        Self {
            default_timeout_seconds: 300,
            auto_detect_silent: true,
            kill_processes_before_uninstall: true,
            create_restore_point: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeftoverConfig {
    pub aggressiveness: Aggressiveness,
    pub confidence_threshold: f32,
    pub scan_appdata: bool,
    pub scan_registry: bool,
    pub scan_services: bool,
    pub scan_tasks: bool,
}

impl Default for LeftoverConfig {
    fn default() -> Self {
        Self {
            aggressiveness: Aggressiveness::Normal,
            confidence_threshold: 0.7,
            scan_appdata: true,
            scan_registry: true,
            scan_services: true,
            scan_tasks: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Aggressiveness {
    Conservative,
    Normal,
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub backup_registry: bool,
    pub move_to_recycle_bin: bool,
    pub max_backup_size_mb: u64,
    pub backup_location: String,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            backup_registry: true,
            move_to_recycle_bin: false,
            max_backup_size_mb: 100,
            backup_location: "auto".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub protected_paths: Vec<String>,
    pub require_confirmation_for_system_apps: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            protected_paths: vec![
                "C:\\Windows".to_string(),
                "C:\\Program Files\\WindowsApps".to_string(),
                "C:\\Windows\\System32".to_string(),
            ],
            require_confirmation_for_system_apps: true,
        }
    }
}

/// Theme configuration for TUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub background: String,
    pub foreground: String,
    pub accent: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
    pub muted: String,
    pub selection_bg: String,
    pub selection_fg: String,
    pub border: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "greek-blue".to_string(),
            background: "#1a1a2e".to_string(),
            foreground: "#e0e0e0".to_string(),
            accent: "#0D5EAF".to_string(),
            success: "#4CAF50".to_string(),
            warning: "#FFC107".to_string(),
            danger: "#F44336".to_string(),
            muted: "#757575".to_string(),
            selection_bg: "#0D5EAF".to_string(),
            selection_fg: "#ffffff".to_string(),
            border: "#444444".to_string(),
        }
    }
}

/// Events emitted during operations
#[derive(Debug, Clone)]
pub enum AppEvent {
    ScanStarted {
        scanner_id: String,
    },
    ScanProgress {
        scanner_id: String,
        current: usize,
        total: usize,
    },
    ScanCompleted {
        scanner_id: String,
        count: usize,
    },
    UninstallStarted {
        app_id: Uuid,
        app_name: String,
    },
    UninstallProgress {
        app_id: Uuid,
        message: String,
    },
    UninstallCompleted {
        app_id: Uuid,
        result: UninstallResult,
    },
    LeftoverScanStarted {
        app_id: Uuid,
    },
    LeftoverFound {
        app_id: Uuid,
        artifact: LeftoverArtifact,
    },
    BatchProgress {
        completed: usize,
        total: usize,
        current_app: String,
    },
    Error {
        operation: String,
        error: String,
    },
}

/// Platform-agnostic system statistics for the TUI status bar.
/// Concrete collectors live in greek-windows (Windows) and greek-platform (Linux/macOS).
#[derive(Debug, Clone, Default)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub disks: Vec<DiskStat>,
    pub gpu: Option<GpuStat>,
    pub battery: Option<BatteryStat>,
    pub uptime_secs: u64,
    pub process_count: usize,
    /// Per-process resource usage keyed by lowercase exe path (Windows only).
    pub processes: HashMap<String, ProcessUsage>,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessUsage {
    pub pid: u32,
    pub name: String,
    pub exe_path: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub virtual_memory: u64,
    pub run_time_secs: u64,
    pub started_at: Option<u64>,
    pub threads: usize,
    pub read_bytes: u64,
    pub written_bytes: u64,
    pub gpu_usage_pct: f32,
    pub vram_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct DiskStat {
    pub label: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

impl DiskStat {
    pub fn usage_pct(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes as f32 / self.total_bytes as f32) * 100.0
        }
    }
}

/// GPU statistics (Windows only, from performance counters).
#[derive(Debug, Clone, Default)]
pub struct GpuStat {
    pub name: String,
    pub usage_pct: f32,
    pub vram_used_bytes: u64,
    pub vram_total_bytes: u64,
}

/// Battery statistics (Windows only, from WMI).
#[derive(Debug, Clone, Default)]
pub struct BatteryStat {
    pub percent: u8,
    pub charging: bool,
}
