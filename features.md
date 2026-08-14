# Greek Uninstaller — Complete Technical Specification
## Rust TUI Application

---

# 1. FEATURES LIST

## 1.1 Core Discovery Engine

| # | Feature | Priority | Description |
|---|---------|----------|-------------|
| 1 | **Registry Scanner** | P0 | Parse `HKLM`/`HKCU` uninstall keys, 32/64-bit hives, MSI GUIDs |
| 2 | **Windows Store Scanner** | P0 | Enumerate UWP/MSIX packages via Windows Management API |
| 3 | **Portable App Detection** | P0 | Heuristic scan of portable directories for orphaned executables |
| 4 | **Browser Extension Mapper** | P1 | Chrome, Firefox, Edge extension enumeration with removal |
| 5 | **Windows Features List** | P1 | Optional OS components (.NET, Hyper-V, WSL) |
| 6 | **Startup Item Detection** | P1 | Registry run keys, startup folders, scheduled startup tasks |
| 7 | **Driver Inventory** | P2 | List `.sys` drivers with metadata and removal capability |
| 8 | **Service Mapper** | P2 | Enumerate Windows services linked to installed apps |
| 9 | **Installation Log Parser** | P2 | Read MSI logs, Inno Setup logs for trace data |
| 10 | **Duplicate Finder** | P3 | Identify multiple versions of same app |

## 1.2 Uninstallation Engine

| # | Feature | Priority | Description |
|---|---------|----------|-------------|
| 11 | **Standard Uninstall** | P0 | Execute official uninstaller with process monitoring |
| 12 | **Silent Uninstall** | P0 | Auto-detect silent flags (`/S`, `/qn`, `/VERYSILENT`) |
| 13 | **Force Remove** | P0 | Bypass uninstaller, delete files/registry directly |
| 14 | **Batch Queue** | P0 | Queue multiple apps, execute sequentially with dependency check |
| 15 | **Process Termination** | P0 | Kill running app processes before uninstall |
| 16 | **Service Stop & Delete** | P1 | Stop dependent services, remove service entries |
| 17 | **Context Menu Cleanup** | P1 | Remove shell extensions during uninstall |
| 18 | **Scheduled Task Removal** | P1 | Delete app-associated scheduled tasks |
| 19 | **Browser Extension Uninstall** | P1 | Remove extensions via browser APIs |
| 20 | **Rollback Support** | P2 | Undo recent uninstall operations |

## 1.3 Leftover Detection ("Greek Scan")

| # | Feature | Priority | Description |
|---|---------|----------|-------------|
| 21 | **File System Orphan Scan** | P0 | Scan `Program Files`, `AppData`, `ProgramData` for orphaned folders |
| 22 | **Registry Orphan Scan** | P0 | Find keys in `HKCU\Software` with no matching installation |
| 23 | **Service Orphan Detection** | P1 | Services with `ImagePath` pointing to missing executables |
| 24 | **Task Orphan Detection** | P1 | Scheduled tasks referencing removed apps |
| 25 | **Shell Extension Orphans** | P1 | Context menu entries for uninstalled software |
| 26 | **Driver Orphan Detection** | P2 | `.sys` files with no active driver entry |
| 27 | **Temp File Cleanup** | P2 | App-specific temp files in `%TEMP%` |
| 28 | **Confidence Scoring** | P2 | ML-based 0.0-1.0 confidence for each leftover artifact |
| 29 | **Cross-Reference Engine** | P2 | Link leftovers to original app via fuzzy matching |
| 30 | **Deep System Scan** | P3 | Full-disk scan for app fingerprints |

## 1.4 TUI Interface

| # | Feature | Priority | Description |
|---|---------|----------|-------------|
| 31 | **Dual-Pane Layout** | P0 | Left: tree/list view; Right: details panel |
| 32 | **Real-Time Fuzzy Search** | P0 | `/` to search, instant filtering with highlighting |
| 33 | **Category Tree View** | P0 | Group by type (Browsers, Development, System, etc.) |
| 34 | **Multi-Select** | P0 | Space to select, batch operations |
| 35 | **Sortable Columns** | P0 | Sort by name, size, install date, publisher |
| 36 | **Preview/Dry-Run Mode** | P0 | Show exactly what will be deleted before action |
| 37 | **Progress Visualization** | P0 | Real-time bars with ETA, throughput |
| 38 | **Operation Log Panel** | P1 | Live tail of uninstall operations |
| 39 | **Keyboard-Driven** | P1 | Vim-style keybindings, customizable shortcuts |
| 40 | **Color Themes** | P1 | Built-in themes + custom TOML theme files |
| 41 | **Help Overlay** | P1 | Contextual `?` help with all keybindings |
| 42 | **Notification Toast** | P2 | Desktop notification on operation completion |
| 43 | **Export View** | P2 | JSON/CSV export of current view |
| 44 | **Mini-Map** | P3 | Scroll position indicator for large lists |

## 1.5 Safety & Recovery

| # | Feature | Priority | Description |
|---|---------|----------|-------------|
| 45 | **System Restore Point** | P0 | Auto-create before any destructive operation |
| 46 | **Registry Backup** | P0 | Export `.reg` files before deletion |
| 47 | **Protected Item Whitelist** | P0 | Block deletion of system-critical paths |
| 48 | **Confirmation Gates** | P0 | Multi-level confirmation based on risk score |
| 49 | **Recycle Bin Option** | P1 | Move to trash instead of permanent delete |
| 50 | **Operation Journal** | P1 | SQLite log of all actions for audit/undo |
| 51 | **Permission Elevation** | P1 | Auto-detect need for admin, request UAC |
| 52 | **File Lock Detection** | P2 | Warn if files are in use by other processes |
| 53 | **Integrity Verification** | P2 | Verify uninstaller signatures before execution |
| 54 | **Sandbox Detection** | P3 | Detect if running in VM/sandbox |

## 1.6 CLI Mode

| # | Feature | Priority | Description |
|---|---------|----------|-------------|
| 55 | **Headless Uninstall** | P1 | `greek uninstall "App Name" --silent` |
| 56 | **List Command** | P1 | `greek list --format json` |
| 57 | **Scan Command** | P1 | `greek scan --leftovers --app "App Name"` |
| 58 | **Batch File Input** | P2 | `greek batch --file apps.txt` |
| 59 | **Report Generation** | P2 | `greek report --output report.html` |
| 60 | **Shell Completions** | P2 | Auto-generate bash/zsh/fish completions |

---

# 2. FUTURE SCOPE

## 2.1 Version 2.0 (6 months post-launch)
- **Remote Uninstall**: Uninstall apps on remote machines via SSH/WinRM
- **Network Discovery**: Scan LAN for installed software inventory
- **Plugin System**: WASM-based plugins for custom uninstallers
- **Cloud Sync**: Sync app lists and settings across devices
- **Dark Web Monitor**: Check if installed apps have known CVEs

## 2.2 Version 3.0 (12 months)
- **AI-Powered Detection**: Train on millions of uninstall patterns for better leftover detection
- **Container Awareness**: Detect and uninstall Docker images/containers
- **WSL Integration**: Manage Linux apps inside WSL from Windows TUI
- **Enterprise Dashboard**: Web dashboard for fleet management
- **Policy Enforcement**: Group policies for allowed/blocked software

## 2.3 Platform Expansion
- **Linux Full Support**: dpkg, rpm, pacman, flatpak, snap backends
- **macOS Full Support**: .app bundles, Homebrew, MacPorts
- **Android Bridge**: ADB-based app management (experimental)

## 2.4 Ecosystem
- **Community Database**: Crowdsourced leftover patterns
- **Integration APIs**: CI/CD pipeline integration for clean build environments
- **Package Manager Wrapper**: Act as a unified frontend for all package managers

---

# 3. PHASEWISE PLAN

## Phase 1: Foundation (Weeks 1–3)
**Goal**: Project structure, core models, basic registry scanning

| Week | Deliverable |
|------|-------------|
| 1 | Workspace scaffolding, CI/CD, `greek-common` crate with all data models |
| 2 | `greek-windows` registry scanner, basic app enumeration |
| 3 | `greek-tui` skeleton: list view, navigation, search box |

**Exit Criteria**: Can launch TUI and see list of installed apps from registry.

## Phase 2: Uninstall Core (Weeks 4–6)
**Goal**: Execute uninstallers, monitor progress, basic removal

| Week | Deliverable |
|------|-------------|
| 4 | Uninstall string parser & executor, process monitoring |
| 5 | Silent flag detection (MSI, Inno, NSIS, InstallShield) |
| 6 | Force remove engine: file deletion, registry deletion, process kill |

**Exit Criteria**: Can uninstall a standard app and a force-removed app via TUI.

## Phase 3: The Greek Engine (Weeks 7–10)
**Goal**: Leftover detection, preview mode, safety features

| Week | Deliverable |
|------|-------------|
| 7 | File system orphan scanner with path heuristics |
| 8 | Registry orphan scanner, service/task orphan detection |
| 9 | Preview/dry-run mode, system restore point integration |
| 10 | Registry backup before delete, protected item whitelist |

**Exit Criteria**: Can run "Greek Scan" on uninstalled app and see leftover artifacts with confidence scores.

## Phase 4: TUI Polish (Weeks 11–13)
**Goal**: Professional-grade interface with all power features

| Week | Deliverable |
|------|-------------|
| 11 | Batch queue system, multi-select operations, progress bars |
| 12 | Category tree view, sortable columns, themes, help overlay |
| 13 | Export/import, operation journal, undo system |

**Exit Criteria**: TUI feels complete; all P0/P1 features implemented.

## Phase 5: CLI & Packaging (Weeks 14–16)
**Goal**: Headless mode, distribution, documentation

| Week | Deliverable |
|------|-------------|
| 14 | `greek-cli` with all core commands |
| 15 | Installer (Inno Setup for Windows), cargo-dist release pipeline |
| 16 | User documentation, API docs, README, tutorial videos |

**Exit Criteria**: `cargo install greek-uninstaller` works; Windows installer available.

## Phase 6: Stabilization (Weeks 17–20)
**Goal**: Bug fixes, performance optimization, community feedback

| Week | Focus |
|------|-------|
| 17-18 | Real-world testing with 100+ popular apps |
| 19 | Performance optimization (parallel scanning, caching) |
| 20 | Final polish, v1.0 release |

---

# 4. WORKFLOW

## 4.1 User Workflow: Standard Uninstall

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Launch     │────▶│  TUI Loads  │────▶│  Registry   │
│  greek-tui  │     │  App List   │     │  Scan       │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
┌─────────────┐     ┌─────────────┐     ┌─────▼───────┐
│  Progress   │◀────│  Execute    │◀────│  Select App │
│  Monitor    │     │  Uninstall  │     │  + Confirm  │
└──────┬──────┘     └─────────────┘     └─────────────┘
       │
       ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Post-      │────▶│  Greek Scan │────▶│  Show       │
│  Uninstall  │     │  (Optional) │     │  Leftovers  │
└─────────────┘     └─────────────┘     └─────────────┘
```

## 4.2 User Workflow: Force Remove

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Select     │────▶│  Press 'F'  │────▶│  Preview    │
│  App        │     │  (Force)    │     │  Panel      │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
┌─────────────┐     ┌─────────────┐     ┌─────▼───────┐
│  Complete   │◀────│  Delete     │◀────│  Confirm    │
│  + Report   │     │  Artifacts  │     │  (High Risk │
└─────────────┘     └─────────────┘     │  Warning)   │
                                        └─────────────┘
```

## 4.3 User Workflow: Batch Uninstall

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Multi-     │────▶│  Press 'B'  │────▶│  Review     │
│  Select     │     │  (Batch)    │     │  Queue      │
│  (Space)    │     │             │     │             │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
┌─────────────┐     ┌─────────────┐     ┌─────▼───────┐
│  Summary    │◀────│  Sequential │◀────│  Execute    │
│  Report     │     │  Execution  │     │  Queue      │
└─────────────┘     └─────────────┘     └─────────────┘
```

## 4.4 Internal Workflow: Uninstall Execution

```
┌─────────────────┐
│  User Confirms  │
└────────┬────────┘
         ▼
┌─────────────────┐     ┌─────────────────┐
│  Create System  │────▶│  Backup Registry│
│  Restore Point  │     │  Keys to .reg   │
└─────────────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│  Detect Install │────▶│  Map to Silent  │
│  Type (MSI/etc) │     │  Flags          │
└─────────────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│  Spawn Process  │────▶│  Monitor with   │
│  with Timeout   │     │  Timeout (5min) │
└─────────────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│  Capture Exit   │────▶│  Verify Removal │
│  Code + Logs    │     │  (Files Gone?)  │
└─────────────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐
│  Offer Greek    │
│  Scan for       │
│  Leftovers      │
└─────────────────┘
```

## 4.5 Internal Workflow: Leftover Scan

```
┌─────────────────┐
│  App Uninstalled│
│  or Selected    │
└────────┬────────┘
         ▼
┌─────────────────┐
│  Gather App     │
│  Metadata       │
│  (name, paths)  │
└────────┬────────┘
         ▼
┌─────────────────────────────────────────┐
│  PARALLEL SCAN (rayon thread pool)      │
│  ├── File System: AppData, ProgramData  │
│  ├── Registry: HKCU, HKLM heuristic     │
│  ├── Services: ImagePath matching       │
│  └── Tasks: Action path matching        │
└────────┬────────────────────────────────┘
         ▼
┌─────────────────┐     ┌─────────────────┐
│  Score Each     │────▶│  Filter by      │
│  Artifact       │     │  Confidence >   │
│  (0.0 - 1.0)    │     │  Threshold      │
└─────────────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐
│  Present to User│
│  with Safety    │
│  Level + Size   │
└─────────────────┘
```

---

# 5. USAGE

## 5.1 TUI Mode (Primary)

```bash
# Launch interactive TUI
greek

# Launch with specific theme
greek --theme dark-blue

# Launch scanning only user-level apps (no admin)
greek --user-only

# Launch and immediately search
greek --search "chrome"
```

### TUI Keybindings

| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Navigate list |
| `←/→` or `h/l` | Collapse/expand categories |
| `Space` | Select/deselect |
| `Enter` | Open action menu |
| `u` | Uninstall selected |
| `f` | Force remove selected |
| `l` | Run Greek Scan |
| `b` | Add to batch queue |
| `B` | View batch queue |
| `/` | Search |
| `s` | Cycle sort (name/size/date) |
| `S` | Filter by source |
| `d` | Toggle details panel |
| `p` | Preview/dry-run mode |
| `Ctrl+r` | Create restore point |
| `Ctrl+u` | Undo last operation |
| `e` | Export current view |
| `?` | Help overlay |
| `q` or `Ctrl+c` | Quit |

## 5.2 CLI Mode (Headless)

```bash
# List all installed apps
greek list
greek list --format json
greek list --format csv --output apps.csv

# Search for an app
greek search "firefox"

# Uninstall an app
greek uninstall "Mozilla Firefox"
greek uninstall --id "{GUID}" --silent
greek uninstall "Firefox" --force --yes

# Batch uninstall from file
greek batch --file to-uninstall.txt --silent

# Scan for leftovers of an uninstalled app
greek scan --leftovers --app "Firefox"
greek scan --leftovers --all --export leftovers.json

# Force remove leftovers
greek clean --leftovers --app "Firefox" --yes

# Create system restore point
greek restore-point --description "Before cleanup"

# Generate report
greek report --output report.html

# Show app details
greek info "Mozilla Firefox"

# Check for updates
greek update --check
```

## 5.3 Configuration File

```toml
# ~/.config/greek/config.toml (Linux/macOS)
# %APPDATA%\Greek\config.toml (Windows)

[ui]
theme = "greek-blue"
show_icons = true
confirm_destructive = true
animation_fps = 30

[scanner]
scan_portable_dirs = ["C:\\Tools", "D:\\Portable"]
scan_browser_extensions = true
scan_windows_features = false
scan_startup_items = true

[uninstall]
default_timeout_seconds = 300
auto_detect_silent = true
kill_processes_before_uninstall = true
create_restore_point = true

[leftover]
aggressiveness = "normal"  # conservative, normal, aggressive
confidence_threshold = 0.7
scan_appdata = true
scan_registry = true
scan_services = true
scan_tasks = true

[backup]
backup_registry = true
move_to_recycle_bin = false
max_backup_size_mb = 100
backup_location = "auto"  # or explicit path

[safety]
protected_paths = [
    "C:\\Windows",
    "C:\\Program Files\\WindowsApps",
    "C:\\Windows\\System32",
]
require_confirmation_for_system_apps = true
```

---

# 6. SETUP

## 6.1 Development Environment

### Prerequisites
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Windows: Visual Studio Build Tools or full VS with C++ workload
# Linux: build-essential, libssl-dev, pkg-config
# macOS: Xcode Command Line Tools
```

### Clone & Build
```bash
git clone https://github.com/greek/greek-uninstaller.git
cd greek-uninstaller

# Build entire workspace
cargo build --workspace

# Build release (optimized)
cargo build --workspace --release

# Run TUI in development
cargo run -p greek-tui

# Run CLI
cargo run -p greek-cli -- --help
```

### Development Scripts
```bash
# Watch mode (auto-rebuild on change)
cargo watch -p greek-tui -x 'run -p greek-tui'

# Run all tests
cargo test --workspace --all-features

# Run clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format check
cargo fmt --all -- --check

# Security audit
cargo audit

# License check
cargo deny check

# Generate docs
cargo doc --workspace --no-deps --open
```

## 6.2 Release Build

### Windows
```bash
# Static CRT linking (no VC++ redist needed)
cargo build --release -p greek-tui --target x86_64-pc-windows-msvc

# Create installer with Inno Setup
iscc scripts/installer.iss
```

### Linux
```bash
cargo build --release -p greek-tui --target x86_64-unknown-linux-gnu

# Create .deb package
cargo deb -p greek-tui

# Create .rpm package
cargo generate-rpm -p greek-tui
```

### macOS
```bash
cargo build --release -p greek-tui --target aarch64-apple-darwin

# Create .app bundle
cargo bundle --release
```

## 6.3 Installation (End User)

### Windows (Recommended)
```powershell
# Using winget
winget install GreekUninstaller

# Using scoop
scoop install greek-uninstaller

# Using MSI installer
msiexec /i GreekUninstaller-1.0.0-x64.msi
```

### Linux
```bash
# Using cargo
cargo install greek-uninstaller

# Using package manager
sudo dpkg -i greek-uninstaller_1.0.0_amd64.deb
```

### macOS
```bash
brew install greek-uninstaller
```

---

# 7. API DESIGN

## 7.1 Public API Surface (`greek-core`)

### Core Traits

```rust
// crates/greek-core/src/traits.rs

/// Trait for any component that can discover installed applications
#[async_trait]
pub trait AppScanner: Send + Sync {
    /// Unique identifier for this scanner
    fn scanner_id(&self) -> &'static str;
    
    /// Human-readable name
    fn scanner_name(&self) -> String;
    
    /// Scan for installed applications
    async fn scan(&self) -> Result<Vec<InstalledApp>, ScanError>;
    
    /// Whether this scanner requires elevated privileges
    fn requires_elevation(&self) -> bool;
}

/// Trait for uninstallation strategies
#[async_trait]
pub trait UninstallStrategy: Send + Sync {
    /// Strategy identifier
    fn strategy_id(&self) -> &'static str;
    
    /// Check if this strategy can handle the given app
    fn can_handle(&self, app: &InstalledApp) -> bool;
    
    /// Execute uninstallation
    async fn uninstall(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult, UninstallError>;
    
    /// Attempt silent uninstallation
    async fn uninstall_silent(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult, UninstallError>;
}

/// Trait for leftover artifact detection
#[async_trait]
pub trait LeftoverAnalyzer: Send + Sync {
    /// Analyze an app for leftover artifacts
    async fn analyze(
        &self,
        app: &InstalledApp,
    ) -> Result<Vec<LeftoverArtifact>, AnalysisError>;
    
    /// Analyze the entire system for orphaned artifacts
    async fn analyze_system(
        &self,
    ) -> Result<Vec<LeftoverArtifact>, AnalysisError>;
}
```

### Core Structs & Enums

```rust
// crates/greek-common/src/models.rs

use chrono::NaiveDate;
use std::path::PathBuf;
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallSource {
    Registry {
        hive: RegistryHive,
        key_path: String,
        is_64_bit: bool,
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

/// Options for uninstallation operations
#[derive(Debug, Clone, Default)]
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

/// Result of an uninstallation operation
#[derive(Debug, Clone)]
pub struct UninstallResult {
    pub app_id: Uuid,
    pub success: bool,
    pub strategy_used: String,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub files_deleted: Vec<PathBuf>,
    pub registry_keys_deleted: Vec<String>,
    pub services_stopped: Vec<String>,
    pub errors: Vec<String>,
    pub restore_point_id: Option<String>,
}

/// Batch operation queue
#[derive(Debug, Clone)]
pub struct BatchQueue {
    pub items: Vec<BatchItem>,
    pub options: UninstallOptions,
}

#[derive(Debug, Clone)]
pub struct BatchItem {
    pub app: InstalledApp,
    pub status: BatchStatus,
    pub result: Option<UninstallResult>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BatchStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Skipped,
}
```

### Core Service API

```rust
// crates/greek-core/src/app_service.rs

pub struct GreekAppService {
    scanners: Vec<Box<dyn AppScanner>>,
    uninstall_strategies: Vec<Box<dyn UninstallStrategy>>,
    leftover_analyzers: Vec<Box<dyn LeftoverAnalyzer>>,
    config: GreekConfig,
    event_bus: broadcast::Sender<AppEvent>,
}

impl GreekAppService {
    /// Create a new service instance with default scanners
    pub fn new(config: GreekConfig) -> Result<Self, ServiceError>;
    
    /// Register a custom scanner
    pub fn register_scanner(&mut self, scanner: Box<dyn AppScanner>);
    
    /// Register a custom uninstall strategy
    pub fn register_strategy(&mut self, strategy: Box<dyn UninstallStrategy>);
    
    /// Register a custom leftover analyzer
    pub fn register_analyzer(&mut self, analyzer: Box<dyn LeftoverAnalyzer>);
    
    /// Scan all sources for installed applications
    pub async fn scan_all_apps(&self) -> Result<Vec<InstalledApp>, ScanError>;
    
    /// Scan a specific source
    pub async fn scan_by_source(
        &self,
        source_type: InstallSourceType,
    ) -> Result<Vec<InstalledApp>, ScanError>;
    
    /// Get detailed info about a specific app
    pub async fn get_app_details(&self, app_id: Uuid) -> Result<InstalledApp, ServiceError>;
    
    /// Uninstall a single application
    pub async fn uninstall_app(
        &self,
        app_id: Uuid,
        options: UninstallOptions,
    ) -> Result<UninstallResult, UninstallError>;
    
    /// Force remove an application
    pub async fn force_remove_app(
        &self,
        app_id: Uuid,
        options: UninstallOptions,
    ) -> Result<UninstallResult, UninstallError>;
    
    /// Analyze leftovers for an app
    pub async fn analyze_leftovers(
        &self,
        app_id: Uuid,
    ) -> Result<Vec<LeftoverArtifact>, AnalysisError>;
    
    /// Clean up leftovers
    pub async fn clean_leftovers(
        &self,
        artifact_ids: Vec<Uuid>,
        options: UninstallOptions,
    ) -> Result<CleanupResult, CleanupError>;
    
    /// Create a batch queue
    pub fn create_batch(&self, options: UninstallOptions) -> BatchQueue;
    
    /// Execute a batch queue
    pub async fn execute_batch(
        &self,
        batch: &mut BatchQueue,
    ) -> Result<Vec<UninstallResult>, BatchError>;
    
    /// Create a system restore point
    pub async fn create_restore_point(
        &self,
        description: &str,
    ) -> Result<String, RestoreError>;
    
    /// Subscribe to real-time events
    pub fn subscribe_events(&self) -> broadcast::Receiver<AppEvent>;
}

/// Events emitted during operations
#[derive(Debug, Clone)]
pub enum AppEvent {
    ScanStarted { scanner_id: String },
    ScanProgress { scanner_id: String, current: usize, total: usize },
    ScanCompleted { scanner_id: String, count: usize },
    UninstallStarted { app_id: Uuid, app_name: String },
    UninstallProgress { app_id: Uuid, message: String },
    UninstallCompleted { app_id: Uuid, result: UninstallResult },
    LeftoverScanStarted { app_id: Uuid },
    LeftoverFound { app_id: Uuid, artifact: LeftoverArtifact },
    BatchProgress { completed: usize, total: usize, current_app: String },
    Error { operation: String, error: String },
}
```

### Windows-Specific API

```rust
// crates/greek-windows/src/lib.rs

pub struct WindowsScanner {
    scan_store_apps: bool,
    scan_32bit: bool,
    scan_64bit: bool,
}

impl AppScanner for WindowsScanner {
    // Implementation...
}

pub struct MsiUninstallStrategy;
pub struct InnoUninstallStrategy;
pub struct NsisUninstallStrategy;
pub struct InstallShieldUninstallStrategy;
pub struct WindowsStoreUninstallStrategy;

pub struct WindowsLeftoverAnalyzer {
    scan_depth: ScanDepth,
}

pub struct SystemRestore {
    pub fn create(description: &str) -> Result<String, windows::core::Error>;
    pub fn list() -> Result<Vec<RestorePoint>, windows::core::Error>;
}

pub struct RegistryBackup {
    pub fn export_key(key_path: &str, output_path: &Path) -> Result<(), RegistryError>;
    pub fn import_reg_file(path: &Path) -> Result<(), RegistryError>;
}

pub struct ServiceManager;
pub struct TaskScheduler;
```

## 7.2 TUI Internal API

```rust
// crates/greek-tui/src/app.rs

pub struct TuiApp {
    service: Arc<GreekAppService>,
    state: AppState,
    ui_state: UiState,
    event_rx: mpsc::Receiver<TuiEvent>,
    should_quit: bool,
}

pub struct AppState {
    pub apps: Vec<InstalledApp>,
    pub filtered_apps: Vec<InstalledApp>,
    pub selected_app: Option<Uuid>,
    pub selected_apps: HashSet<Uuid>,
    pub batch_queue: BatchQueue,
    pub current_operation: Option<OperationState>,
    pub logs: Vec<LogEntry>,
    pub last_error: Option<String>,
}

pub struct UiState {
    pub active_panel: Panel,
    pub show_help: bool,
    pub show_preview: bool,
    pub sort_by: SortColumn,
    pub sort_order: SortOrder,
    pub filter_text: String,
    pub filter_source: Option<InstallSourceType>,
    pub theme: Theme,
    pub notification: Option<Notification>,
}

pub enum Panel {
    AppList,
    Details,
    BatchQueue,
    LeftoverPreview,
    LogViewer,
    Settings,
}

pub enum TuiEvent {
    Tick,
    Key(KeyEvent),
    AppEvent(AppEvent),
    Resize(u16, u16),
}
```

## 7.3 CLI API

```rust
// crates/greek-cli/src/commands.rs

#[derive(Parser)]
#[command(name = "greek")]
#[command(about = "The uninstaller that actually uninstalls")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    #[arg(short, long, global = true)]
    pub verbose: bool,
    
    #[arg(short, long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List installed applications
    List {
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        source: Option<InstallSourceType>,
    },
    
    /// Search for applications
    Search {
        query: String,
        #[arg(short, long)]
        fuzzy: bool,
    },
    
    /// Uninstall an application
    Uninstall {
        #[arg(required = true)]
        app: String,
        #[arg(short, long)]
        silent: bool,
        #[arg(short, long)]
        force: bool,
        #[arg(short, long)]
        yes: bool,
        #[arg(long)]
        timeout: Option<u64>,
    },
    
    /// Scan for leftover artifacts
    Scan {
        #[arg(long, group = "scan_type")]
        leftovers: bool,
        #[arg(long, group = "scan_type")]
        all: bool,
        #[arg(short, long)]
        app: Option<String>,
        #[arg(short, long)]
        export: Option<PathBuf>,
    },
    
    /// Clean leftover artifacts
    Clean {
        #[arg(short, long)]
        leftovers: bool,
        #[arg(short, long)]
        app: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    
    /// Batch uninstall from file
    Batch {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long)]
        silent: bool,
    },
    
    /// Create system restore point
    RestorePoint {
        #[arg(short, long, default_value = "Greek Uninstaller Restore Point")]
        description: String,
    },
    
    /// Generate system report
    Report {
        #[arg(short, long, default_value = "report.html")]
        output: PathBuf,
    },
    
    /// Show application details
    Info {
        app: String,
    },
}
```

---

# 8. BACKEND WORKING PLAN

## 8.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         TUI Layer                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ App List │  │ Details  │  │ Preview  │  │  Batch   │   │
│  │  Panel   │  │  Panel   │  │  Panel   │  │  Queue   │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
│       └──────────────┴─────────────┴─────────────┘          │
│                         │                                   │
│                    Event Bus (tokio::sync::broadcast)       │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────┼───────────────────────────────────┐
│                    Service Layer                            │
│  ┌──────────────────────┴──────────────────────────────┐   │
│  │              GreekAppService                         │   │
│  │  ┌─────────┐ ┌─────────────┐ ┌──────────────────┐  │   │
│  │  │ Scanner │ │  Uninstall  │ │ Leftover Analyzer│  │   │
│  │  │ Engine  │ │   Engine    │ │     Engine       │  │   │
│  │  └────┬────┘ └──────┬──────┘ └────────┬─────────┘  │   │
│  └───────┼─────────────┼─────────────────┼────────────┘   │
└──────────┼─────────────┼─────────────────┼────────────────┘
           │             │                 │
┌──────────┼─────────────┼─────────────────┼────────────────┐
│          │      Platform Abstraction Layer                │
│  ┌───────▼─────┐  ┌────▼──────┐  ┌──────▼──────┐        │
│  │   Windows   │  │   Linux   │  │    macOS    │        │
│  │   Module    │  │   Module  │  │    Module   │        │
│  │  (greek-    │  │  (greek-  │  │   (greek-   │        │
│  │  windows)   │  │  platform)│  │  platform)  │        │
│  └──────┬──────┘  └─────┬─────┘  └──────┬──────┘        │
│         │               │               │                │
│  ┌──────▼───────────────▼───────────────▼──────┐        │
│  │         Operating System APIs               │        │
│  │  Registry | WMI | Services | File System    │        │
│  └─────────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────────┘
```

## 8.2 Scanning Engine

### 8.2.1 Registry Scanner (Windows)

```rust
pub struct RegistryScanner {
    hives: Vec<RegistryHive>,
}

impl AppScanner for RegistryScanner {
    async fn scan(&self) -> Result<Vec<InstalledApp>, ScanError> {
        let mut apps = Vec::new();
        
        // Scan both 32-bit and 64-bit registry views
        for hive in &self.hives {
            let uninstall_key = format!("{}\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall", 
                hive.as_str());
            
            // Open key with enumeration permission
            let key = RegKey::predef(hive.to_hkey())
                .open_subkey_with_flags(&uninstall_key, KEY_READ | KEY_ENUMERATE_SUB_KEYS)?;
            
            // Enumerate subkeys (each is an installed app)
            for subkey_name in key.enum_keys() {
                let subkey_name = subkey_name?;
                let app_key = key.open_subkey(&subkey_name)?;
                
                // Parse app entry
                if let Some(app) = self.parse_app_entry(&app_key, &subkey_name, *hive).await? {
                    // Skip system components unless explicitly enabled
                    if !app.is_system_component || self.include_system {
                        apps.push(app);
                    }
                }
            }
        }
        
        // Deduplicate by name+version
        apps.sort_by(|a, b| a.name.cmp(&b.name));
        apps.dedup_by(|a, b| a.name == b.name && a.version == b.version);
        
        Ok(apps)
    }
}
```

### 8.2.2 Parallel Scanning Strategy

```rust
pub async fn parallel_scan(
    scanners: Vec<Box<dyn AppScanner>>,
) -> Result<Vec<InstalledApp>, ScanError> {
    let mut handles = Vec::new();
    
    for scanner in scanners {
        let handle = tokio::spawn(async move {
            scanner.scan().await
        });
        handles.push(handle);
    }
    
    let mut all_apps = Vec::new();
    for handle in handles {
        let apps = handle.await??;
        all_apps.extend(apps);
    }
    
    // Merge and deduplicate results from multiple scanners
    merge_scan_results(all_apps)
}
```

## 8.3 Uninstall Engine

### 8.3.1 Strategy Pattern Implementation

```rust
pub struct UninstallEngine {
    strategies: Vec<Box<dyn UninstallStrategy>>,
}

impl UninstallEngine {
    pub async fn uninstall(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult, UninstallError> {
        // 1. Find the best strategy
        let strategy = self.strategies.iter()
            .find(|s| s.can_handle(app))
            .ok_or(UninstallError::NoStrategyFound)?;
        
        // 2. Pre-uninstall: kill processes, stop services
        if options.kill_processes {
            self.kill_app_processes(app).await?;
        }
        
        // 3. Create restore point
        let restore_point = if options.create_restore_point {
            Some(SystemRestore::create(&format!("Before uninstalling {}", app.name)).await?)
        } else {
            None
        };
        
        // 4. Backup registry
        if options.backup_registry {
            for key in &app.registry_keys {
                RegistryBackup::export_key(&key.path, &backup_path).await?;
            }
        }
        
        // 5. Execute uninstallation
        let result = if options.silent && app.quiet_uninstall_string.is_some() {
            strategy.uninstall_silent(app, options.clone()).await
        } else {
            strategy.uninstall(app, options.clone()).await
        };
        
        // 6. Post-uninstall verification
        if let Ok(ref mut res) = result {
            res.success = self.verify_uninstall(app).await?;
        }
        
        result
    }
}
```

### 8.3.2 MSI Strategy

```rust
pub struct MsiUninstallStrategy;

impl UninstallStrategy for MsiUninstallStrategy {
    fn can_handle(&self, app: &InstalledApp) -> bool {
        matches!(app.source, InstallSource::Registry { .. }) &&
        app.uninstall_string.as_ref()
            .map(|s| s.contains("MsiExec"))
            .unwrap_or(false)
    }
    
    async fn uninstall_silent(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult, UninstallError> {
        // Extract ProductCode from uninstall string
        let product_code = extract_product_code(&app.uninstall_string)?;
        
        let mut cmd = Command::new("msiexec.exe");
        cmd.arg("/x").arg(&product_code)
           .arg("/qn")
           .arg("/norestart");
        
        let output = cmd.output().await?;
        
        Ok(UninstallResult {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            ..Default::default()
        })
    }
}
```

### 8.3.3 Force Remove Strategy

```rust
pub struct ForceRemoveStrategy;

impl UninstallStrategy for ForceRemoveStrategy {
    fn can_handle(&self, _app: &InstalledApp) -> bool {
        true // Force remove can handle anything
    }
    
    async fn uninstall(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult, UninstallError> {
        let mut result = UninstallResult::default();
        
        // 1. Terminate all processes
        let processes = find_processes_by_path(&app.install_location);
        for pid in processes {
            terminate_process(pid).await?;
            result.processes_killed.push(pid);
        }
        
        // 2. Stop and delete services
        let services = find_services_by_path(&app.install_location);
        for service in services {
            ServiceManager::stop(&service).await?;
            ServiceManager::delete(&service).await?;
            result.services_stopped.push(service);
        }
        
        // 3. Delete files and directories
        if let Some(ref location) = app.install_location {
            delete_directory_recursive(location, options.move_to_recycle_bin).await?;
            result.files_deleted.push(location.clone());
        }
        
        // 4. Delete registry keys
        for key in &app.registry_keys {
            Registry::delete_key_recursive(&key.path).await?;
            result.registry_keys_deleted.push(key.path.clone());
        }
        
        // 5. Delete scheduled tasks
        let tasks = find_tasks_by_app(app);
        for task in tasks {
            TaskScheduler::delete(&task).await?;
        }
        
        result.success = true;
        Ok(result)
    }
}
```

## 8.4 Leftover Analyzer Engine

### 8.4.1 Heuristic Scoring

```rust
pub struct HeuristicAnalyzer;

impl LeftoverAnalyzer for HeuristicAnalyzer {
    async fn analyze(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>, AnalysisError> {
        let mut artifacts = Vec::new();
        let keywords = extract_keywords(app);
        
        // Parallel scan using rayon
        let scan_targets = vec![
            scan_appdata(&keywords),
            scan_programdata(&keywords),
            scan_registry(&keywords),
            scan_services(&keywords),
            scan_tasks(&keywords),
        ];
        
        for target in scan_targets {
            let found = target.await?;
            for mut artifact in found {
                // Calculate confidence score
                artifact.confidence = calculate_confidence(&artifact, app);
                if artifact.confidence >= self.threshold {
                    artifacts.push(artifact);
                }
            }
        }
        
        // Sort by confidence descending
        artifacts.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        Ok(artifacts)
    }
}

fn calculate_confidence(artifact: &LeftoverArtifact, app: &InstalledApp) -> f32 {
    let mut score = 0.0f32;
    
    // Name match in path
    if artifact.path.to_string_lossy().to_lowercase()
        .contains(&app.name.to_lowercase()) {
        score += 0.4;
    }
    
    // Publisher match
    if let Some(ref publisher) = app.publisher {
        if artifact.path.to_string_lossy().to_lowercase()
            .contains(&publisher.to_lowercase()) {
            score += 0.2;
        }
    }
    
    // Install date proximity
    if let (Some(install_date), Some(artifact_date)) = (app.install_date, artifact.last_modified) {
        let days_diff = (artifact_date - install_date).num_days().abs();
        if days_diff < 7 {
            score += 0.15;
        }
    }
    
    // Path in known app directories
    if is_in_app_directory(&artifact.path) {
        score += 0.15;
    }
    
    // No other apps reference this
    if !is_referenced_by_other_apps(&artifact.path) {
        score += 0.1;
    }
    
    score.min(1.0)
}
```

## 8.5 Event-Driven Architecture

```rust
pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
    
    pub fn publish(&self, event: AppEvent) {
        let _ = self.sender.send(event); // Ignore lagging receivers
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }
}

// In TUI: Update UI based on events
pub async fn event_loop(
    mut event_rx: mpsc::Receiver<TuiEvent>,
    mut app_event_rx: broadcast::Receiver<AppEvent>,
    app: &mut TuiApp,
) -> Result<(), Box<dyn Error>> {
    loop {
        tokio::select! {
            Some(tui_event) = event_rx.recv() => {
                match tui_event {
                    TuiEvent::Tick => app.on_tick(),
                    TuiEvent::Key(key) => app.on_key(key).await?,
                    TuiEvent::Resize(w, h) => app.on_resize(w, h),
                }
            }
            Ok(app_event) = app_event_rx.recv() => {
                match app_event {
                    AppEvent::ScanProgress { current, total, .. } => {
                        app.update_progress(current, total);
                    }
                    AppEvent::UninstallProgress { app_id, message } => {
                        app.update_operation_log(app_id, message);
                    }
                    AppEvent::Error { operation, error } => {
                        app.show_error(&format!("{}: {}", operation, error));
                    }
                    _ => {}
                }
            }
        }
        
        if app.should_quit {
            break;
        }
    }
    Ok(())
}
```

## 8.6 State Management

```rust
pub struct AppState {
    // Core data
    pub apps: Vec<InstalledApp>,
    pub filtered_apps: Vec<usize>, // indices into apps
    pub selected_index: usize,
    pub selected_apps: HashSet<Uuid>,
    
    // View state
    pub list_state: ListState,
    pub scroll_offset: usize,
    pub sort_column: SortColumn,
    pub sort_order: SortOrder,
    pub filter_query: String,
    
    // Operation state
    pub current_operation: Option<OperationState>,
    pub operation_log: Vec<LogEntry>,
    pub batch_queue: BatchQueue,
    
    // UI state
    pub active_panel: Panel,
    pub show_help: bool,
    pub show_preview: bool,
    pub notification: Option<Notification>,
    pub last_tick: Instant,
}

impl AppState {
    pub fn filter_apps(&mut self) {
        let query = self.filter_query.to_lowercase();
        self.filtered_apps = self.apps.iter().enumerate()
            .filter(|(_, app)| {
                app.name.to_lowercase().contains(&query) ||
                app.publisher.as_ref()
                    .map(|p| p.to_lowercase().contains(&query))
                    .unwrap_or(false)
            })
            .map(|(idx, _)| idx)
            .collect();
        
        self.sort_apps();
    }
    
    pub fn sort_apps(&mut self) {
        let apps = &self.apps;
        self.filtered_apps.sort_by(|a, b| {
            let app_a = &apps[*a];
            let app_b = &apps[*b];
            
            let ordering = match self.sort_column {
                SortColumn::Name => app_a.name.cmp(&app_b.name),
                SortColumn::Size => app_a.size_bytes.cmp(&app_b.size_bytes),
                SortColumn::Date => app_a.install_date.cmp(&app_b.install_date),
                SortColumn::Publisher => app_a.publisher.cmp(&app_b.publisher),
            };
            
            match self.sort_order {
                SortOrder::Ascending => ordering,
                SortOrder::Descending => ordering.reverse(),
            }
        });
    }
}
```

## 8.7 Persistence Layer

```rust
pub struct OperationJournal {
    db: SqlitePool,
}

impl OperationJournal {
    pub async fn new(path: &Path) -> Result<Self, sqlx::Error> {
        let db = SqlitePool::connect(&path.to_string_lossy()).await?;
        
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS operations (
                id TEXT PRIMARY KEY,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                operation_type TEXT NOT NULL,
                app_id TEXT,
                app_name TEXT,
                success BOOLEAN,
                details TEXT,
                restore_point_id TEXT
            );
            
            CREATE TABLE IF NOT EXISTS deleted_artifacts (
                id TEXT PRIMARY KEY,
                operation_id TEXT,
                artifact_type TEXT,
                original_path TEXT,
                backup_path TEXT,
                FOREIGN KEY (operation_id) REFERENCES operations(id)
            );
            "#
        ).execute(&db).await?;
        
        Ok(Self { db })
    }
    
    pub async fn log_operation(&self, entry: JournalEntry) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO operations (id, operation_type, app_id, app_name, success, details, restore_point_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#
        )
        .bind(&entry.id)
        .bind(&entry.operation_type)
        .bind(&entry.app_id)
        .bind(&entry.app_name)
        .bind(entry.success)
        .bind(&entry.details)
        .bind(&entry.restore_point_id)
        .execute(&self.db).await?;
        
        Ok(())
    }
    
    pub async fn get_undoable_operations(&self, limit: usize) -> Result<Vec<JournalEntry>, sqlx::Error> {
        sqlx::query_as::<_, JournalEntry>(
            r#"
            SELECT * FROM operations 
            WHERE success = true 
            ORDER BY timestamp DESC 
            LIMIT ?1
            "#
        )
        .bind(limit as i64)
        .fetch_all(&self.db).await
    }
}
```

## 8.8 Performance Considerations

| Concern | Solution |
|---------|----------|
| Registry scan speed | Parallel hive enumeration, cache results |
| File system scan | `jwalk` for parallel directory walking, skip system paths |
| Large app lists | Virtual scrolling in TUI (render only visible items) |
| Memory usage | Streaming scan results, don't hold everything in memory |
| UI responsiveness | All I/O on tokio threads, TUI on main thread |
| Startup time | Lazy loading of scanners, background initial scan |

## 8.9 Error Handling Strategy

```rust
// greek-common/src/errors.rs

#[derive(Error, Debug)]
pub enum GreekError {
    #[error("Scan failed: {0}")]
    ScanError(#[from] ScanError),
    
    #[error("Uninstall failed: {0}")]
    UninstallError(#[from] UninstallError),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Operation timed out after {0}s")]
    Timeout(u64),
    
    #[error("App not found: {0}")]
    AppNotFound(String),
    
    #[error("Force remove blocked: {0} is a protected system component")]
    ProtectedSystemComponent(String),
    
    #[error("Registry operation failed: {0}")]
    RegistryError(#[from] RegistryError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Top-level result type
pub type Result<T> = std::result::Result<T, GreekError>;
```

---

This specification provides a complete blueprint for building Greek Uninstaller. Every module has defined interfaces, data flows are documented, and the architecture supports both TUI and CLI modes from a shared core. The backend is designed for async operation with proper error handling, event streaming, and state management suitable for a responsive terminal application.