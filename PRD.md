# Product Requirements Document (PRD)
## Greek Uninstaller — Complete System Uninstaller
### Rust TUI Application

---

## 1. Executive Summary

**Greek Uninstaller** is a comprehensive, terminal-based software uninstallation tool built in pure Rust. It provides deep system scanning, intelligent leftover detection, batch operations, and force-removal capabilities — all through a modern, keyboard-driven Terminal User Interface (TUI). The name "Greek" signifies thoroughness: it leaves no trace behind.

**Target Platforms:** Windows 10/11 (primary), Linux (secondary), macOS (secondary)

---

## 2. Product Vision

> *"The uninstaller that actually uninstalls."*

Most system uninstallers leave behind registry entries, hidden folders, service entries, and scheduled tasks. Greek Uninstaller performs **deep forensic-style removal** with an intelligent scanning engine that identifies orphaned artifacts across the entire system.

---

## 3. Core Features

### 3.1 Program Discovery & Cataloging
| Feature | Description |
|---------|-------------|
| **Registry Scanner** | Reads `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall` and `HKCU` equivalents |
| **Windows Store Apps** | Enumerates UWP/MSIX packages via PowerShell APIs |
| **Portable App Detection** | Scans common portable directories and identifies executable bundles |
| **Browser Extensions** | Lists Chrome, Firefox, Edge extensions with removal capability |
| **Windows Features** | Optional components, .NET frameworks, Windows capabilities |
| **Import Detection** | Reads installation logs (MSI, Inno Setup, NSIS) for trace data |

### 3.2 Uninstallation Engine
| Feature | Description |
|---------|-------------|
| **Standard Uninstall** | Executes official uninstaller strings with timeout handling |
| **Silent Uninstall** | Auto-detects silent flags (`/S`, `/quiet`, `/qn`) and executes unattended |
| **Force Remove** | Terminates processes, stops services, removes files without official uninstaller |
| **Batch Uninstall** | Queue multiple apps; execute sequentially with dependency checking |
| **Uninstall Monitoring** | Real-time tracking of uninstaller process with log capture |

### 3.3 Leftover Detection (The "Greek" Scan)
| Feature | Description |
|---------|-------------|
| **File System Scan** | Searches `Program Files`, `AppData`, `ProgramData` for orphaned folders |
| **Registry Deep Scan** | Finds orphaned keys in `HKCR`, `HKLM\System`, services, drivers |
| **Service Detection** | Identifies Windows services with no associated executable |
| **Scheduled Tasks** | Finds tasks pointing to removed applications |
| **Shell Extensions** | Detects context menu entries for uninstalled software |
| **Driver Cleanup** | Identifies orphaned .sys files and driver registry entries |

### 3.4 TUI Interface Features
| Feature | Description |
|---------|-------------|
| **Dual-Pane Layout** | Left: App list with tree view; Right: Details/Preview panel |
| **Real-Time Search** | Fuzzy find with instant filtering (`/`, `?`, `Ctrl+F`) |
| **Sort & Filter** | By size, install date, publisher, drive usage |
| **Preview Mode** | Dry-run showing exactly what will be deleted |
| **Progress Visualization** | Real-time progress bars with ETA during operations |
| **Undo Queue** | Transaction log allowing rollback of recent changes |
| **Export/Import** | JSON/CSV export of installed apps; import for remote auditing |

### 3.5 Safety & Recovery
| Feature | Description |
|---------|-------------|
| **System Restore Point** | Auto-create restore point before any removal (Windows) |
| **Backup Registry Keys** | Export related registry branches before deletion |
| **Protected Items** | Whitelist system-critical entries (Windows Update, Drivers) |
| **Confirmation Gates** | Multi-level confirmation for system-level changes |
| **Trash/Recycle** | Move to recycle bin instead of permanent delete (optional) |

---

## 4. Technical Architecture

### 4.1 Crate Structure (Workspace)

```
greek-uninstaller/
├── Cargo.toml                 # Workspace root
├── greek-core/                # Core business logic
│   ├── src/
│   │   ├── scanner/           # Program discovery engines
│   │   ├── uninstaller/       # Removal orchestration
│   │   ├── leftover/          # Orphaned artifact detection
│   │   ├── models/            # Data structures (App, RegistryKey, etc.)
│   │   └── utils/             # Helpers, path resolution, permissions
│   └── Cargo.toml
├── greek-tui/                 # Terminal UI layer
│   ├── src/
│   │   ├── app.rs             # TUI application state machine
│   │   ├── ui/                # Component rendering
│   │   ├── events/            # Input handling, keymaps
│   │   └── widgets/           # Custom ratatui widgets
│   └── Cargo.toml
├── greek-cli/                 # Headless CLI mode
│   └── src/main.rs
├── greek-windows/             # Windows-specific APIs
│   ├── src/
│   │   ├── registry.rs        # WinReg bindings
│   │   ├── wmi.rs             # WMI queries for installed apps
│   │   ├── services.rs        # Service control manager
│   │   └── msi.rs             # Windows Installer API
│   └── Cargo.toml
└── greek-common/              # Shared types and errors
    └── src/
```

### 4.2 Key Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI framework (v0.28+) |
| `crossterm` | Cross-platform terminal I/O |
| `tokio` | Async runtime for non-blocking operations |
| `windows` | Official Microsoft Windows API bindings |
| `winreg` | Windows registry operations |
| `serde` + `serde_json` | Configuration and data serialization |
| `clap` | CLI argument parsing |
| `fuzzy-matcher` | Fuzzy search for app filtering |
| `humansize` | Human-readable file sizes |
| `chrono` | Date/time handling |
| `color-eyre` | Rich error reporting |
| `tracing` + `tracing-subscriber` | Structured logging |
| `tui-textarea` | Multi-line text input for notes |
| `tui-tree-widget` | Hierarchical app tree view |

### 4.3 Data Models

```rust
// Core Application Entry
pub struct InstalledApp {
    pub id: Uuid,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub install_date: Option<NaiveDate>,
    pub install_location: Option<PathBuf>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
    pub size_bytes: Option<u64>,
    pub source: InstallSource,  // Registry, Store, Portable, etc.
    pub icon_path: Option<PathBuf>,
    pub is_system_component: bool,
    pub estimated_leftover_size: Option<u64>,
}

pub enum InstallSource {
    Registry(RegistrySource),
    WindowsStore(String),      // Package Family Name
    Portable(PathBuf),
    BrowserExtension(BrowserType),
    WindowsFeature,
    Scanned,                   // Detected via file system scan
}

pub struct LeftoverArtifact {
    pub id: Uuid,
    pub app_id: Uuid,
    pub artifact_type: ArtifactType,
    pub path: PathBuf,
    pub size_bytes: Option<u64>,
    pub confidence: f32,        // 0.0-1.0 ML-based confidence
    pub safety_level: SafetyLevel,
}

pub enum ArtifactType {
    Directory,
    File,
    RegistryKey,
    RegistryValue,
    Service,
    ScheduledTask,
    ShellExtension,
    Driver,
}
```

### 4.4 Async Architecture

```
┌─────────────────┐
│   TUI Thread    │  ← ratatui render loop (60fps target)
│  (Main Thread)  │
└────────┬────────┘
         │ mpsc channel
         ▼
┌─────────────────┐
│  Event Router   │  ← crossterm events + custom app events
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌───────┐ ┌──────────┐
│Scanner│ │Uninstall │  ← tokio worker threads
│Engine │ │  Engine  │
└───────┘ └──────────┘
    │
    ▼
┌───────────┐
│ Leftover  │  ← Background analysis with progress streaming
│  Analyzer │
└───────────┘
```

---

## 5. TUI Design Specification

### 5.1 Layout (Default View)

```
┌─────────────────────────────────────────────────────────────────────┐
│ Greek Uninstaller v1.0.0                                [?] Help    │
├──────────────────────────────┬──────────────────────────────────────┤
│ 🔍 Search: ________________  │  Details: Mozilla Firefox 127.0      │
│                              │  ─────────────────────────────────   │
│ 📁 Installed (247)           │  Publisher: Mozilla Corporation      │
│ ├── 🌐 Browsers (12)         │  Version: 127.0.1                    │
│ │   ├── Firefox ⭐          │  Size: 245 MB                        │
│ │   ├── Chrome              │  Installed: 2024-03-15               │
│ │   └── Edge                │  Location: C:\Program Files\...      │
│ ├── 🛠️ Development (34)     │                                      │
│ │   ├── Rust Toolchain      │  [Uninstall] [Force Remove] [Scan    │
│ │   ├── VS Code             │   Leftovers]                         │
│ │   └── Docker Desktop      │                                      │
│ └── ...                     │  ⚠️  3 leftover artifacts detected   │
│                              │  ├── AppData\Local\Mozilla\...       │
│ [Space] Select  [Enter] Act  │  └── Registry: HKCU\Software\Moz...  │
│                              │                                      │
├──────────────────────────────┤  Safety: Medium Risk                 │
│ Status: Ready | 2 selected   │  [🔒 Create Restore Point First]    │
│ Total selected size: 1.2 GB  │                                      │
└──────────────────────────────┴──────────────────────────────────────┘
```

### 5.2 Key Bindings

| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Navigate list |
| `←/→` or `h/l` | Collapse/expand categories |
| `Space` | Select/deselect item |
| `a` | Select all visible |
| `Enter` | Open action menu for selected |
| `/` or `Ctrl+F` | Focus search box |
| `s` | Sort cycle (name/size/date) |
| `f` | Filter by source (Registry/Store/All) |
| `d` | Show details panel |
| `l` | Run leftover scan on selected |
| `b` | Batch uninstall queue |
| `u` | Undo last operation |
| `Ctrl+C` or `q` | Quit (with confirmation if operations pending) |
| `?` or `F1` | Help overlay |

### 5.3 Color Scheme (Configurable)

```rust
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,           // Primary action color (Greek blue: #0D5EAF)
    pub success: Color,          // Green for safe operations
    pub warning: Color,          // Yellow for medium risk
    pub danger: Color,           // Red for destructive actions
    pub muted: Color,            // Gray for secondary text
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub border: Color,
}
```

---

## 6. Feature Modules Deep Dive

### 6.1 Scanner Module

**Registry Scanner (Windows)**
- Read both 32-bit and 64-bit registry hives
- Parse `DisplayName`, `DisplayVersion`, `Publisher`, `InstallDate`, `InstallLocation`, `UninstallString`, `QuietUninstallString`
- Handle special cases: MSI products (GUID subkeys), Windows patches (`KB*` entries)
- Cross-reference with `HKCR\Installer\Products` for additional metadata

**Windows Store Scanner**
- PowerShell: `Get-AppxPackage` and `Get-AppxPackageManifest`
- Extract display name from manifest resources
- Handle provisioned packages vs user packages

**Portable Scanner**
- Scan configurable directories (`C:\Portable`, `~/Applications`, etc.)
- Heuristic: directory containing `.exe` with matching name, no registry entry
- SHA256 hash for identification

### 6.2 Uninstaller Module

**Execution Flow:**
1. Validate uninstall string (exists, accessible)
2. Detect installer type (MSI, Inno Setup, NSIS, InstallShield, custom)
3. Map to silent flags:
   - MSI: `/qn /norestart`
   - Inno Setup: `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`
   - NSIS: `/S`
   - InstallShield: `/s /v"/qn"`
4. Spawn process with timeout (default 5 minutes)
5. Monitor for child processes
6. Capture exit code and stdout/stderr
7. Post-uninstall verification (check if files/registry still exist)

**Force Remove:**
- Terminate processes by executable name (using `taskkill` or Windows API)
- Stop and delete services
- Take ownership of protected files (`SeTakeOwnershipPrivilege`)
- Delete files with retry logic (handle "file in use")
- Remove registry keys recursively

### 6.3 Leftover Analyzer

**Heuristic Engine:**
- **Path Matching:** Check if paths contain app name, publisher, or known identifiers
- **Registry Orphans:** Keys in `HKCU\Software` with no corresponding `Program Files` entry
- **File Age:** Recently modified files in temp directories after uninstall date
- **Service Orphans:** Services with `ImagePath` pointing to non-existent executables
- **ML Confidence:** (Future) Train on patterns of known clean vs leftover artifacts

**Safety Levels:**
- `Safe`: User data directories, obvious orphans
- `Caution`: Shared directories, ambiguous matches
- `Dangerous`: System directories, Windows protected paths

---

## 7. Security & Permissions

| Concern | Mitigation |
|---------|-----------|
| **UAC Elevation** | Request admin rights on startup for system-wide operations; degrade gracefully for user-level apps only |
| **Protected System Files** | Maintain whitelist of critical paths (`C:\Windows`, `System32`, `SysWOW64`) |
| **Registry Safety** | Backup `.reg` files before deletion; validate key paths |
| **Code Signing** | Verify uninstaller executables are signed before execution (optional) |
| **Audit Logging** | All operations logged to `%LOCALAPPDATA%\GreekUninstaller\logs\` with timestamps |

---

## 8. Configuration

```toml
# config.toml
[general]
theme = "greek-blue"
confirm_destructive = true
create_restore_points = true
language = "en"

[scanner]
scan_portable_dirs = ["C:\\Tools", "D:\\Portable"]
scan_browser_extensions = true
scan_windows_features = false

[leftover]
aggressiveness = "normal"  # conservative, normal, aggressive
scan_appdata = true
scan_registry = true
scan_services = true

[uninstall]
default_timeout_seconds = 300
auto_detect_silent = true
kill_processes_before_uninstall = false

[backup]
backup_registry_before_delete = true
move_to_recycle_bin = false  # if false, permanent delete
max_backup_size_mb = 100
```

---

## 9. Milestones & Roadmap

### Phase 1: Foundation (Weeks 1-3)
- [ ] Project scaffolding (workspace, CI/CD)
- [ ] Core data models and error handling
- [ ] Basic registry scanner (Windows)
- [ ] TUI skeleton with ratatui (list view, basic navigation)

### Phase 2: Core Uninstall (Weeks 4-6)
- [ ] Uninstall string execution with process monitoring
- [ ] Silent install flag detection for major installer types
- [ ] Progress reporting in TUI
- [ ] Basic details panel

### Phase 3: The "Greek" Engine (Weeks 7-9)
- [ ] File system leftover scanner
- [ ] Registry orphan detector
- [ ] Service and scheduled task scanner
- [ ] Preview/dry-run mode
- [ ] Backup and restore point integration

### Phase 4: Polish & Power Features (Weeks 10-12)
- [ ] Batch uninstall queue
- [ ] Fuzzy search and advanced filtering
- [ ] Export/import functionality
- [ ] Custom themes
- [ ] CLI mode (`greek --uninstall "App Name" --force`)

### Phase 5: Cross-Platform (Weeks 13-16)
- [ ] Linux: dpkg/rpm/pacman package backend
- [ ] macOS: Homebrew and .app bundle detection
- [ ] Unified abstraction over platform differences

---

## 10. Success Metrics

| Metric | Target |
|--------|--------|
| App Discovery Accuracy | >98% of Control Panel "Programs and Features" entries |
| Silent Uninstall Success Rate | >85% for supported installer types |
| Leftover Detection Precision | >90% true positive rate |
| TUI Responsiveness | <16ms render time, no blocking on main thread |
| Binary Size | <10MB release build |
| Memory Usage | <100MB during full system scan |

---

## 11. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Registry corruption | Critical | Always backup; whitelist system keys; test in VM |
| False positive leftovers | Medium | Confidence scoring; user review before deletion |
| Anti-virus false positives | Medium | Code sign binary; avoid suspicious APIs |
| Complex installer edge cases | Low | Extensive test suite with real-world apps |

---

## 12. Appendix

### A. Competitive Analysis
- **Revo Uninstaller**: GUI-only, Windows-only, proprietary
- **IObit Uninstaller**: Heavy, ad-supported, closed source
- **Bulk Crap Uninstaller**: Open source, GUI-only, .NET-based
- **Greek Differentiator**: Pure Rust (memory safe, fast), TUI-first (lightweight, SSH-friendly), cross-platform potential

### B. Testing Strategy
- Unit tests for all scanner parsers
- Integration tests with real installer packages in CI
- Property-based testing for path resolution
- Manual testing matrix: Windows 10/11, various installer types

---

**Document Version:** 1.0  
**Author:** Product Team  
**Date:** 2026-08-13  
**Status:** Draft for Review

---

This PRD provides a complete blueprint. The modular workspace architecture lets you start with `greek-core` (the scanner) and `greek-tui` (the interface) in parallel. Would you like me to elaborate on any specific module — perhaps the Windows registry scanner implementation or the ratatui component architecture?