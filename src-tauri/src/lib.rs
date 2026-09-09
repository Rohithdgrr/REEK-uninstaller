use base64::Engine;
use greek_common::models::{InstalledApp, InstallSource, RegistryHive};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

// ---------- Shared state ----------
/// JSON-serialized scan payload with a timestamp for TTL caching.
struct CachedScans {
    at: std::time::Instant,
    payload: String,
}

impl CachedScans {
    fn fresh(&self) -> bool {
        self.at.elapsed() < std::time::Duration::from_secs(60)
    }
}

struct AppRegistry {
    apps: Mutex<HashMap<String, InstalledApp>>,
    videos: Mutex<Option<CachedScans>>,
    dev_modules: Mutex<Option<CachedScans>>,
    leftovers: Mutex<HashMap<String, CachedScans>>,
}

impl AppRegistry {
    fn new() -> Self {
        Self {
            apps: Mutex::new(HashMap::new()),
            videos: Mutex::new(None),
            dev_modules: Mutex::new(None),
            leftovers: Mutex::new(HashMap::new()),
        }
    }

    fn invalidate_leftovers_for(&self, id: &str) {
        if let Ok(mut m) = self.leftovers.lock() {
            m.remove(id);
        }
    }
}

// ---------- DTOs ----------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub size_bytes: Option<u64>,
    pub size_display: Option<String>,
    pub install_date: Option<String>,
    pub install_location: Option<String>,
    pub source_label: String,
    pub icon_path: Option<String>,
    pub icon_color: Option<String>,
    /// True when REEK has a real removal path for this app (vendor
    /// uninstaller or native Store/Portable/Package removal). The desktop
    /// list only shows deletable apps; the frontend also filters on this
    /// as defense-in-depth so a stale cache can never surface a
    /// non-deletable row.
    #[serde(default = "default_true")]
    pub can_uninstall: bool,
    /// True when a real extracted icon PNG is already cached for this app.
    /// Rows with false still attempt on-demand extraction via get_app_icon,
    /// so every visible row converges to a real high-quality icon instead
    /// of initials.
    #[serde(default)]
    pub has_icon: bool,
}

fn default_true() -> bool {
    true
}

impl From<InstalledApp> for AppEntry {
    fn from(a: InstalledApp) -> Self {
        let source_label = match &a.source {
            InstallSource::Registry { .. } => "Registry",
            InstallSource::WindowsStore { .. } => "Store",
            InstallSource::Portable { .. } => "Portable",
            InstallSource::BrowserExtension { .. } => "Extension",
            InstallSource::WindowsFeature { .. } => "Feature",
            InstallSource::PackageManager { manager, .. } => match manager {
                greek_common::models::PackageManager::Winget => "Winget",
                _ => "Package",
            },
        }
        .to_string();
        let has_icon = a.icon_path.is_some();
        let can_uninstall = a.has_removal_path();
        Self {
            id: a.id.to_string(),
            name: a.name.clone(),
            publisher: a.publisher.clone(),
            version: a.version.clone(),
            size_bytes: a.size_bytes,
            size_display: a.display_size(),
            install_date: a.install_date.map(|d| d.to_string()),
            install_location: a.install_location.as_ref().map(|p| p.to_string_lossy().to_string()),
            source_label,
            icon_path: a.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            icon_color: a.metadata.get("icon_color").cloned(),
            can_uninstall,
            has_icon,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDetails {
    pub id: String,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub size_bytes: Option<u64>,
    pub size_display: Option<String>,
    pub install_date: Option<String>,
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
    pub source_label: String,
    pub is_system: bool,
    pub registry_keys: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub icon_path: Option<String>,
    pub icon_color: Option<String>,
}

impl From<InstalledApp> for AppDetails {
    fn from(a: InstalledApp) -> Self {
        let source_label = match &a.source {
            InstallSource::Registry { .. } => "Registry",
            InstallSource::WindowsStore { .. } => "Store",
            InstallSource::Portable { .. } => "Portable",
            InstallSource::BrowserExtension { .. } => "Extension",
            InstallSource::WindowsFeature { .. } => "Feature",
            InstallSource::PackageManager { manager, .. } => match manager {
                greek_common::models::PackageManager::Winget => "Winget",
                _ => "Package",
            },
        }
        .to_string();
        Self {
            id: a.id.to_string(),
            name: a.name.clone(),
            publisher: a.publisher.clone(),
            version: a.version.clone(),
            size_bytes: a.size_bytes,
            size_display: a.display_size(),
            install_date: a.install_date.map(|d| d.to_string()),
            install_location: a.install_location.as_ref().map(|p| p.to_string_lossy().to_string()),
            uninstall_string: a.uninstall_string.clone(),
            quiet_uninstall_string: a.quiet_uninstall_string.clone(),
            source_label,
            is_system: a.is_system_component,
            registry_keys: a.registry_keys.iter().map(|k| k.path.clone()).collect(),
            metadata: a.metadata.clone(),
            icon_path: a.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            icon_color: a.metadata.get("icon_color").cloned(),
        }
    }
}

// ---------- Validation helpers (Audit 2 §1.1) ----------
const MAX_UNINSTALL_IDS: usize = 100;
const MAX_PATHS_PER_REQUEST: usize = 500;
const MAX_PATH_LEN: usize = 4096;

fn validate_uuid_str(id: &str) -> Result<uuid::Uuid, String> {
    let t = id.trim();
    if t.is_empty() || t.len() > 64 {
        return Err(format!("Invalid id length: {}", t.len()));
    }
    uuid::Uuid::parse_str(t).map_err(|_| format!("Invalid app id (not a UUID): {t}"))
}

fn validate_ids(ids: &[String]) -> Result<(), String> {
    if ids.is_empty() {
        return Err("No applications selected".into());
    }
    if ids.len() > MAX_UNINSTALL_IDS {
        return Err(format!("Too many applications (max {MAX_UNINSTALL_IDS})"));
    }
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        validate_uuid_str(id)?;
        if !seen.insert(id) {
            return Err(format!("Duplicate id: {id}"));
        }
    }
    Ok(())
}

fn validate_single_id(id: &str) -> Result<(), String> {
    validate_uuid_str(id).map(|_| ())
}

fn validate_paths(paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("No paths provided".into());
    }
    if paths.len() > MAX_PATHS_PER_REQUEST {
        return Err(format!("Too many paths (max {MAX_PATHS_PER_REQUEST})"));
    }
    for p in paths {
        let t = p.trim();
        if t.is_empty() || t.len() > MAX_PATH_LEN {
            return Err(format!("Invalid path length: {}", t.len()));
        }
        if t.contains('\0') {
            return Err("Path contains null byte".into());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UninstallPayload {
    pub ids: Vec<String>,
    pub force: bool,
    pub silent: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallProgressPayload {
    pub seq: u64,
    pub current: usize,
    pub total: usize,
    pub app_name: String,
    pub status: String,
    pub log: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallResultDto {
    pub id: String,
    pub name: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatsDto {
    pub cpu: f32,
    pub ram_used: u64,
    pub ram_total: u64,
    pub ram_pct: f32,
    pub swap_used: u64,
    pub swap_total: u64,
    pub disks: Vec<DiskDto>,
    pub gpu: Option<GpuDto>,
    pub battery: Option<BatteryDto>,
    pub uptime_secs: u64,
    pub process_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskDto {
    pub label: String,
    pub used: u64,
    pub total: u64,
    pub pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDto {
    pub name: String,
    pub usage: f32,
    pub vram_used: u64,
    pub vram_total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryDto {
    pub percent: u8,
    pub charging: bool,
}

impl SystemStatsDto {
    fn from_common(s: greek_common::SystemStats) -> Self {
        Self {
            cpu: s.cpu_usage,
            ram_used: s.ram_used_bytes,
            ram_total: s.ram_total_bytes,
            ram_pct: if s.ram_total_bytes > 0 { s.ram_used_bytes as f32 / s.ram_total_bytes as f32 * 100.0 } else { 0.0 },
            swap_used: s.swap_used_bytes,
            swap_total: s.swap_total_bytes,
            disks: s.disks.into_iter().map(|d| {
                let pct = d.usage_pct();
                DiskDto { label: d.label.clone(), used: d.used_bytes, total: d.total_bytes, pct }
            }).collect(),
            gpu: s.gpu.map(|g| GpuDto { name: g.name, usage: g.usage_pct, vram_used: g.vram_used_bytes, vram_total: g.vram_total_bytes }),
            battery: s.battery.map(|b| BatteryDto { percent: b.percent, charging: b.charging }),
            uptime_secs: s.uptime_secs,
            process_count: s.process_count,
        }
    }
    fn fallback() -> Self {
        Self { cpu: 0.0, ram_used: 0, ram_total: 0, ram_pct: 0.0, swap_used: 0, swap_total: 0, disks: vec![], gpu: None, battery: None, uptime_secs: 0, process_count: 0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeftoverDto {
    pub id: String,
    pub artifact_type: String,
    pub path: String,
    pub size_bytes: Option<u64>,
    pub size_display: Option<String>,
    pub confidence: f32,
    pub safety: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppResourceDto {
    pub is_running: bool,
    pub pid: Option<u32>,
    pub process_count: usize,
    pub cpu: f32,
    pub memory_bytes: u64,
    pub memory_display: Option<String>,
    pub gpu: f32,
    pub vram_bytes: u64,
    pub exe_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEntryDto {
    pub id: String,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub size_display: String,
    pub drive: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevModuleDto {
    pub id: String,
    pub path: String,
    pub name: String,
    pub kind: String,
    pub language: String,
    pub size_bytes: u64,
    pub size_display: String,
    pub file_count: usize,
    pub drive: String,
}

// ---------- Helpers ----------
fn mock_apps() -> Vec<InstalledApp> {
    let mk = |name: &str, version: &str, publisher: &str, size_mb: u64, date: &str| {
        let mut a = InstalledApp::new(
            name.to_string(),
            InstallSource::Registry {
                hive: RegistryHive::Hklm,
                key_path: format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", name),
            },
        );
        a.version = Some(version.to_string());
        a.publisher = Some(publisher.to_string());
        a.size_bytes = Some(size_mb * 1024 * 1024);
        a.install_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok();
        a.uninstall_string = Some(format!("\"C:\\Program Files\\{}\\uninstall.exe\" /S", name));
        a.quiet_uninstall_string = Some(format!("\"C:\\Program Files\\{}\\uninstall.exe\" /S", name));
        a.install_location = Some(std::path::PathBuf::from(format!("C:\\Program Files\\{}", name)));
        a
    };
    vec![
        mk("Google Chrome", "127.0.6533", "Google LLC", 512, "2024-08-12"),
        mk("Visual Studio Code", "1.92.2", "Microsoft Corporation", 348, "2024-07-30"),
        mk("Node.js", "20.16.0", "OpenJS Foundation", 89, "2024-06-18"),
        mk("Docker Desktop", "4.33.1", "Docker Inc.", 2100, "2024-05-02"),
        mk("Mozilla Firefox", "128.0", "Mozilla Corporation", 245, "2024-08-01"),
        mk("Spotify", "1.2.42", "Spotify AB", 420, "2024-03-15"),
        mk("Slack", "4.39.95", "Slack Technologies", 310, "2024-07-10"),
        mk("VLC media player", "3.0.21", "VideoLAN", 95, "2024-04-22"),
        mk("Git", "2.46.0", "The Git Development Community", 320, "2024-08-05"),
        mk("Python 3.12", "3.12.4", "Python Software Foundation", 180, "2024-06-10"),
        mk("7-Zip", "24.07", "Igor Pavlov", 8, "2024-02-11"),
        mk("Notion", "3.2.1", "Notion Labs", 270, "2024-07-01"),
    ]
}

async fn real_scan() -> Result<Vec<InstalledApp>, String> {
    use greek_common::GreekConfig;
    use greek_core::GreekAppService;
    let config = GreekConfig::default();
    let mut svc = GreekAppService::new(config).map_err(|e| e.to_string())?;
    svc.scan_all_apps().await.map_err(|e| e.to_string())
}

// ---------- Commands ----------
#[tauri::command]
async fn scan_applications(registry: State<'_, AppRegistry>) -> Result<Vec<AppEntry>, String> {
    // On Windows, never mask a real scan failure with mock data:
    // fake C:\Program Files entries can never uninstall and look like app bugs.
    // Mocks stay dev-only for non-Windows targets.
    let mut apps = match real_scan().await {
        Ok(v) if !v.is_empty() => v,
        Ok(_) => {
            if cfg!(target_os = "windows") {
                return Err("Scan returned 0 applications. Try running as Administrator, then Scan again.".into());
            } else { mock_apps() }
        }
        Err(e) => {
            if cfg!(target_os = "windows") {
                return Err(format!("Scan failed ({e}). Try running as Administrator, then Scan again."));
            }
            eprintln!("[scan] failed {e}, fallback mock (non-Windows dev)");
            mock_apps()
        }
    };
    // Defense-in-depth: hide OS-critical apps even if service cache is bypassed
    // (e.g. mock fallback). Mirrors filter in GreekAppService::scan_all_apps.
    let before = apps.len();
    apps.retain(|a| a.is_safe_to_show());
    if before != apps.len() {
        eprintln!("[scan] filtered {} OS-critical apps (showing {})", before - apps.len(), apps.len());
    }
    // Same for entries with no supported removal path: the desktop list only
    // shows applications that can actually be deleted.
    let before_removal = apps.len();
    apps.retain(|a| a.has_removal_path());
    if before_removal != apps.len() {
        eprintln!("[scan] filtered {} apps with no removal path (showing {})", before_removal - apps.len(), apps.len());
    }
    {
        let mut map = registry.apps.lock().map_err(|e| format!("lock {e}"))?;
        map.clear();
        for a in &apps { map.insert(a.id.to_string(), a.clone()); }
    }
    Ok(apps.into_iter().map(AppEntry::from).collect())
}

#[tauri::command]
async fn get_app_details(registry: State<'_, AppRegistry>, id: String) -> Result<AppDetails, String> {
    validate_single_id(&id)?;
    let map = registry.apps.lock().map_err(|e| format!("lock {e}"))?;
    let app = map.get(&id).ok_or_else(|| format!("App {id} not found. Re-scan."))?;
    Ok(AppDetails::from(app.clone()))
}

#[tauri::command]
async fn get_system_stats() -> Result<SystemStatsDto, String> {
    // Offload blocking 250ms+ PS call to blocking thread
    let dto = tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let mut c = greek_windows::SystemStatsCollector::new();
            let s = c.collect();
            SystemStatsDto::from_common(s)
        }
        #[cfg(not(target_os = "windows"))]
        {
            SystemStatsDto::fallback()
        }
    })
    .await
    .map_err(|e| format!("join {e}"))?;
    Ok(dto)
}

fn leftover_to_dto(a: greek_common::LeftoverArtifact) -> LeftoverDto {
    let size_display = a.size_bytes.map(|b| humansize::format_size(b, humansize::BINARY));
    LeftoverDto {
        id: a.id.to_string(),
        artifact_type: format!("{:?}", a.artifact_type),
        path: a.path.to_string_lossy().to_string(),
        size_bytes: a.size_bytes,
        size_display,
        confidence: a.confidence,
        safety: format!("{:?}", a.safety_level),
        description: Some(a.description.clone()),
    }
}

#[tauri::command]
async fn analyze_leftovers(
    registry: State<'_, AppRegistry>,
    id: String,
    refresh: Option<bool>,
) -> Result<Vec<LeftoverDto>, String> {
    validate_single_id(&id)?;
    // 60s TTL cache: repeat drawer opens must not re-walk the device.
    // Manual Rescan passes refresh=true to bypass it.
    if !refresh.unwrap_or(false) {
        if let Ok(map) = registry.leftovers.lock() {
            if let Some(hit) = map.get(&id) {
                if hit.fresh() {
                    if let Ok(dtos) = serde_json::from_str::<Vec<LeftoverDto>>(&hit.payload) {
                        return Ok(dtos);
                    }
                }
            }
        }
    }
    let app = {
        let map = registry.apps.lock().map_err(|e| format!("lock {e}"))?;
        map.get(&id).cloned().ok_or_else(|| format!("App {id} not found"))?
    };
    use greek_common::GreekConfig;
    use greek_core::GreekAppService;
    let config = GreekConfig::default();
    let mut svc = GreekAppService::new(config).map_err(|e| e.to_string())?;
    let artifacts = svc.analyze_leftovers(&app).await.map_err(|e| e.to_string())?;
    let dtos: Vec<LeftoverDto> = artifacts.into_iter().map(leftover_to_dto).collect();
    if let Ok(payload) = serde_json::to_string(&dtos) {
        if let Ok(mut map) = registry.leftovers.lock() {
            map.insert(id, CachedScans { at: std::time::Instant::now(), payload });
        }
    }
    Ok(dtos)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanLeftoverItem {
    pub path: String,
    pub artifact_type: String,
    pub safety: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanLeftoverResultDto {
    pub path: String,
    pub deleted: bool,
    pub reason: String,
}

fn validate_clean_items(items: &[CleanLeftoverItem]) -> Result<(), String> {
    if items.is_empty() {
        return Err("No leftover items provided".into());
    }
    if items.len() > MAX_PATHS_PER_REQUEST {
        return Err(format!("Too many items (max {MAX_PATHS_PER_REQUEST})"));
    }
    for it in items {
        let t = it.path.trim();
        if t.is_empty() || t.len() > MAX_PATH_LEN {
            return Err(format!("Invalid path length: {}", t.len()));
        }
        if t.contains('\0') {
            return Err("Path contains null byte".into());
        }
    }
    Ok(())
}

#[tauri::command]
async fn clean_leftover_artifacts(
    registry: State<'_, AppRegistry>,
    items: Vec<CleanLeftoverItem>,
    force: bool,
) -> Result<Vec<CleanLeftoverResultDto>, String> {
    validate_clean_items(&items)?;
    use greek_common::GreekConfig;
    use greek_core::{CleanLeftoverRequest, GreekAppService};
    let config = GreekConfig::default();
    let svc = GreekAppService::new(config).map_err(|e| e.to_string())?;
    // Explicit user-confirmed clean deletes regardless of safety level
    // (protected paths are still always refused); auto-clean passes
    // only_safe = true via the internal path instead.
    let reqs: Vec<CleanLeftoverRequest> = items
        .into_iter()
        .map(|i| CleanLeftoverRequest {
            path: i.path,
            artifact_type: i.artifact_type,
            safety: i.safety,
        })
        .collect();
    let outcomes = svc.clean_leftover_paths(reqs, !force).await;
    // Cleaned results change what a rescan shows: drop cached scans.
    if let Ok(mut m) = registry.leftovers.lock() {
        m.clear();
    }
    Ok(outcomes
        .into_iter()
        .map(|o| CleanLeftoverResultDto {
            path: o.path,
            deleted: o.deleted,
            reason: o.reason,
        })
        .collect())
}

#[tauri::command]
async fn uninstall_applications(
    app_handle: AppHandle,
    registry: State<'_, AppRegistry>,
    payload: UninstallPayload,
) -> Result<Vec<UninstallResultDto>, String> {
    validate_ids(&payload.ids)?;
    let total = payload.ids.len();
    let targets: Vec<InstalledApp> = {
        let map = registry.apps.lock().map_err(|e| format!("lock {e}"))?;
        let mut out = Vec::new();
        for id in &payload.ids {
            if let Some(a) = map.get(id) {
                // Block OS-critical uninstalls even if user somehow selected them
                if a.is_os_critical() {
                    eprintln!("[uninstall] blocked OS-critical app: {}", a.name);
                    continue;
                }
                out.push(a.clone());
            } else { eprintln!("[uninstall] missing {id}"); }
        }
        if out.is_empty() { return Err("Selected apps not found or are OS-critical and cannot be removed.".into()); }
        out
    };
    use greek_common::{GreekConfig, UninstallOptions};
    use greek_core::{CleanLeftoverRequest, GreekAppService};
    let config = GreekConfig::default();
    let mut svc = GreekAppService::new(config).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    let mut seq: u64 = 0;
    // Rate limiting: ensure at most 100 events/sec (10ms min interval). Since uninstall is sequential
    // this naturally throttles, but we enforce it explicitly for safety (§1.3).
    let mut last_emit = std::time::Instant::now() - std::time::Duration::from_millis(100);
    let min_interval = std::time::Duration::from_millis(10);
    for (idx, app) in targets.iter().enumerate() {
        let cur = idx + 1;
        seq += 1;
        // Throttle: sleep if emitting too fast
        let elapsed = last_emit.elapsed();
        if elapsed < min_interval {
            tokio::time::sleep(min_interval - elapsed).await;
        }
        let _ = app_handle.emit("uninstall-progress", UninstallProgressPayload {
            seq, current: cur, total, app_name: app.name.clone(), status: "processing".into(),
            log: format!("Uninstalling {} {}{}", app.name, app.version.as_deref().unwrap_or(""), if payload.force { " (force)" } else { "" })
        });
        last_emit = std::time::Instant::now();
        let mut opts = if payload.force { UninstallOptions::force() } else { UninstallOptions::standard() };
        opts.silent = payload.silent.unwrap_or(false);
        let res = if payload.force { svc.force_remove_app(app, opts.clone()).await } else { svc.uninstall_app(app, opts.clone()).await };
        match res {
            Ok(r) => {
                // Auto-clean Safe leftovers after a successful removal so
                // "uninstall" actually uninstalls (non-Safe items stay for
                // explicit cleaning via clean_leftover_artifacts).
                let mut leftover_note = String::new();
                if r.success {
                    match svc.analyze_leftovers(app).await {
                        Ok(arts) => {
                            let safe: Vec<CleanLeftoverRequest> = arts
                                .into_iter()
                                .filter(|a| a.is_safe_to_delete())
                                .map(|a| CleanLeftoverRequest {
                                    path: a.path.to_string_lossy().to_string(),
                                    artifact_type: format!("{:?}", a.artifact_type),
                                    safety: format!("{:?}", a.safety_level),
                                })
                                .collect();
                            if !safe.is_empty() {
                                let n = safe.len();
                                let outcomes = svc.clean_leftover_paths(safe, true).await;
                                let cleaned = outcomes.iter().filter(|o| o.deleted).count();
                                leftover_note = format!(", leftovers:{cleaned}/{n}");
                                registry.invalidate_leftovers_for(&app.id.to_string());
                            }
                        }
                        Err(e) => {
                            leftover_note = format!(", leftover-scan skipped ({e})");
                        }
                    }
                }
                let log = if r.success { format!("{} via {} (files:{}, regs:{}{})", app.name, r.strategy_used, r.files_deleted.len(), r.registry_keys_deleted.len(), leftover_note) } else { r.errors.join("; ") };
                let status = if r.success { "done" } else { "error" };
                seq += 1;
                let elapsed2 = last_emit.elapsed();
                if elapsed2 < min_interval {
                    tokio::time::sleep(min_interval - elapsed2).await;
                }
                let _ = app_handle.emit("uninstall-progress", UninstallProgressPayload { seq, current: cur, total, app_name: app.name.clone(), status: status.into(), log: log.clone() });
                last_emit = std::time::Instant::now();
                results.push(UninstallResultDto { id: app.id.to_string(), name: app.name.clone(), success: r.success, error: if r.success { None } else { Some(log) } });
            }
            Err(e) => {
                let msg = e.to_string();
                seq += 1;
                let elapsed2 = last_emit.elapsed();
                if elapsed2 < min_interval {
                    tokio::time::sleep(min_interval - elapsed2).await;
                }
                let _ = app_handle.emit("uninstall-progress", UninstallProgressPayload { seq, current: cur, total, app_name: app.name.clone(), status: "error".into(), log: msg.clone() });
                last_emit = std::time::Instant::now();
                results.push(UninstallResultDto { id: app.id.to_string(), name: app.name.clone(), success: false, error: Some(msg) });
            }
        }
    }
    // Final summary event with seq ensures frontend can detect completion even if prior events were batched (§2.1)
    seq += 1;
    let _ = app_handle.emit("uninstall-progress", UninstallProgressPayload {
        seq, current: total, total, app_name: "complete".into(), status: "completed".into(),
        log: format!("Batch complete: {} succeeded, {} failed", results.iter().filter(|r| r.success).count(), results.iter().filter(|r| !r.success).count())
    });
    {
        let mut map = registry.apps.lock().map_err(|e| format!("lock {e}"))?;
        for r in &results {
            if r.success {
                map.remove(&r.id);
                registry.invalidate_leftovers_for(&r.id);
            }
        }
    }
    Ok(results)
}

/// Read a cached icon PNG off disk and base64 it for the WebView.
async fn serve_icon_file(path: std::path::PathBuf) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }
    // Offload blocking read + base64 to thread pool
    let bytes = tokio::task::spawn_blocking(move || std::fs::read(&path))
        .await
        .map_err(|e| format!("join {e}"))?
        .map_err(|e| format!("read icon {e}"))?;
    // Limit to reasonable size (e.g. 2MB) to avoid huge payloads
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("icon too large".into());
    }
    // Reject truncated/corrupt caches instead of serving broken images.
    if bytes.len() < 64 {
        return Ok(None);
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(Some(b64))
}

#[tauri::command]
async fn get_app_icon(registry: State<'_, AppRegistry>, id: String) -> Result<Option<String>, String> {
    validate_single_id(&id)?;
    let app_opt = {
        let map = registry.apps.lock().map_err(|e| format!("lock {e}"))?;
        map.get(&id).cloned()
    };
    let Some(app) = app_opt else {
        return Ok(None);
    };
    // Fast path: previously extracted PNG.
    if let Some(path) = app.icon_path.clone() {
        if path.exists() {
            return serve_icon_file(path).await;
        }
    }
    // On-demand fallback: extract a real icon for apps the scan-time pass
    // missed, so every visible row converges to a real icon instead of
    // initials. Result lands in the on-disk PNG cache and the registry map.
    #[cfg(target_os = "windows")]
    {
        let mut one = app.clone();
        let extracted = tokio::task::spawn_blocking(move || {
            let extractor = greek_windows::icon::IconExtractor::new();
            extractor.extract_icons(std::slice::from_mut(&mut one));
            if let Some(p) = one.icon_path.clone() {
                let color = greek_windows::icon::IconExtractor::dominant_color(&p)
                    .map(|(r, g, b)| format!("{r},{g},{b}"));
                Some((p, color))
            } else {
                None
            }
        })
        .await
        .map_err(|e| format!("join {e}"))?;
        if let Some((path, color)) = extracted {
            if let Ok(mut map) = registry.apps.lock() {
                if let Some(entry) = map.get_mut(&id) {
                    entry.icon_path = Some(path.clone());
                    if let Some(c) = color {
                        entry.metadata.entry("icon_color".into()).or_insert(c);
                    }
                }
            }
            return serve_icon_file(path).await;
        }
    }
    Ok(None)
}

fn resource_for_app(app: &InstalledApp, stats: &greek_common::SystemStats) -> Option<AppResourceDto> {
    let mut total_cpu = 0f32;
    let mut total_mem = 0u64;
    let mut total_gpu = 0f32;
    let mut total_vram = 0u64;
    let mut pids: Vec<u32> = Vec::new();
    let mut exe_example: Option<String> = None;

    let app_name_lower = app.name.to_lowercase();
    let loc_lower = app.install_location.as_ref().map(|p| p.to_string_lossy().to_lowercase().to_string());
    let exe_meta_lower = app.metadata.get("exe_path").map(|s| s.trim_matches('"').to_lowercase());
    let icon_path_lower = app
        .metadata
        .get("display_icon")
        .and_then(|s| {
            let t = s.trim();
            let stripped = match t.rfind(',') {
                Some(pos) if {
                    let suffix = t[pos + 1..].trim();
                    let digits = suffix.strip_prefix('-').unwrap_or(suffix);
                    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
                } => t[..pos].trim(),
                _ => t,
            };
            let clean = stripped.trim_matches('"').trim().to_lowercase();
            if clean.is_empty() { None } else { Some(clean) }
        });

    for proc in stats.processes.values() {
        let exe_lower = proc.exe_path.to_lowercase();
        let name_lower = proc.name.to_lowercase();
        let mut matched = false;

        if let Some(ref exe) = exe_meta_lower {
            if &exe_lower == exe {
                matched = true;
            }
        }
        if !matched {
            if let Some(ref loc) = loc_lower {
                // Directory prefix match (avoid matching C:\Program Files vs C:\Program Files (x86) false positive: require trailing slash or exact)
                if exe_lower.starts_with(loc) {
                    matched = true;
                } else if loc.ends_with(".exe") && &exe_lower == loc {
                    matched = true;
                }
            }
        }
        if !matched {
            if let Some(ref icon) = icon_path_lower {
                if &exe_lower == icon {
                    matched = true;
                }
            }
        }
        if !matched {
            // Name heuristic: need at least 4 chars to avoid "git" vs "digit" false positives
            if app_name_lower.len() >= 4 && name_lower.len() >= 4 {
                let app_norm = app_name_lower.replace(' ', "");
                let proc_norm = name_lower.trim_end_matches(".exe").replace(' ', "");
                if proc_norm.contains(&app_norm) || app_norm.contains(&proc_norm) {
                    matched = true;
                }
            }
        }
        if matched {
            total_cpu += proc.cpu_usage;
            total_mem += proc.memory_bytes;
            total_gpu += proc.gpu_usage_pct;
            total_vram += proc.vram_bytes;
            pids.push(proc.pid);
            if exe_example.is_none() {
                exe_example = Some(proc.exe_path.clone());
            }
        }
    }

    if pids.is_empty() {
        return None;
    }
    pids.sort_unstable();
    pids.dedup();
    Some(AppResourceDto {
        is_running: true,
        pid: pids.first().copied(),
        process_count: pids.len(),
        cpu: total_cpu,
        memory_bytes: total_mem,
        memory_display: Some(humansize::format_size(total_mem, humansize::BINARY)),
        gpu: total_gpu,
        vram_bytes: total_vram,
        exe_path: exe_example,
    })
}

#[tauri::command]
async fn get_app_resources(registry: State<'_, AppRegistry>) -> Result<HashMap<String, AppResourceDto>, String> {
    // Snapshot apps without holding lock during blocking collection
    let apps: Vec<InstalledApp> = {
        let map = registry.apps.lock().map_err(|e| format!("lock {e}"))?;
        map.values().cloned().collect()
    };
    if apps.is_empty() {
        return Ok(HashMap::new());
    }
    let stats = tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let mut c = greek_windows::SystemStatsCollector::new();
            c.collect()
        }
        #[cfg(not(target_os = "windows"))]
        {
            greek_common::SystemStats::default()
        }
    })
    .await
    .map_err(|e| format!("join {e}"))?;

    let mut out = HashMap::new();
    for app in apps {
        if let Some(res) = resource_for_app(&app, &stats) {
            out.insert(app.id.to_string(), res);
        }
    }
    Ok(out)
}

#[tauri::command]
async fn get_app_resource(registry: State<'_, AppRegistry>, id: String) -> Result<Option<AppResourceDto>, String> {
    validate_single_id(&id)?;
    let app = {
        let map = registry.apps.lock().map_err(|e| format!("lock {e}"))?;
        map.get(&id).cloned().ok_or_else(|| format!("App {id} not found"))?
    };
    let stats = tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let mut c = greek_windows::SystemStatsCollector::new();
            c.collect()
        }
        #[cfg(not(target_os = "windows"))]
        {
            greek_common::SystemStats::default()
        }
    })
    .await
    .map_err(|e| format!("join {e}"))?;
    Ok(resource_for_app(&app, &stats))
}

#[tauri::command]
async fn scan_videos(
    registry: State<'_, AppRegistry>,
    refresh: Option<bool>,
) -> Result<Vec<VideoEntryDto>, String> {
    // 60s TTL: tab switches and re-opens must not re-walk the device.
    // Manual Rescan passes refresh=true to bypass it.
    if !refresh.unwrap_or(false) {
        if let Ok(slot) = registry.videos.lock() {
            if let Some(hit) = slot.as_ref() {
                if hit.fresh() {
                    if let Ok(dtos) = serde_json::from_str::<Vec<VideoEntryDto>>(&hit.payload) {
                        return Ok(dtos);
                    }
                }
            }
        }
    }
    use greek_core::video::VideoScanner;
    let scanner = VideoScanner::new();
    let entries = scanner.scan_all().await.map_err(|e| e.to_string())?;
    let dtos: Vec<VideoEntryDto> = entries.into_iter().map(|v| VideoEntryDto {
        id: v.id.to_string(),
        path: v.path.to_string_lossy().to_string(),
        name: v.name,
        extension: v.extension,
        size_bytes: v.size_bytes,
        size_display: v.size_display,
        drive: v.drive,
    }).collect();
    if let Ok(payload) = serde_json::to_string(&dtos) {
        if let Ok(mut slot) = registry.videos.lock() {
            *slot = Some(CachedScans { at: std::time::Instant::now(), payload });
        }
    }
    Ok(dtos)
}

#[tauri::command]
async fn delete_videos(registry: State<'_, AppRegistry>, paths: Vec<String>) -> Result<Vec<String>, String> {
    validate_paths(&paths)?;
    use greek_core::video::VideoScanner;
    let scanner = VideoScanner::new();
    let pbs: Vec<std::path::PathBuf> = paths.into_iter().map(std::path::PathBuf::from).collect();
    let done = scanner.delete_videos(pbs).await.map_err(|e| e.to_string())?;
    // Deletions change what a rescan shows.
    if let Ok(mut slot) = registry.videos.lock() {
        *slot = None;
    }
    Ok(done)
}

#[tauri::command]
async fn scan_dev_modules(
    registry: State<'_, AppRegistry>,
    refresh: Option<bool>,
) -> Result<Vec<DevModuleDto>, String> {
    // 60s TTL: tab switches and re-opens must not re-walk the device.
    // Manual Rescan passes refresh=true to bypass it.
    if !refresh.unwrap_or(false) {
        if let Ok(slot) = registry.dev_modules.lock() {
            if let Some(hit) = slot.as_ref() {
                if hit.fresh() {
                    if let Ok(dtos) = serde_json::from_str::<Vec<DevModuleDto>>(&hit.payload) {
                        return Ok(dtos);
                    }
                }
            }
        }
    }
    use greek_core::dev_modules::DevModulesScanner;
    let scanner = DevModulesScanner::new();
    let entries = scanner.scan_all().await.map_err(|e| e.to_string())?;
    let dtos: Vec<DevModuleDto> = entries.into_iter().map(|m| DevModuleDto {
        id: m.id.to_string(),
        path: m.path.to_string_lossy().to_string(),
        name: m.name,
        kind: m.kind.as_str().to_string(),
        language: m.language,
        size_bytes: m.size_bytes,
        size_display: m.size_display,
        file_count: m.file_count,
        drive: m.drive,
    }).collect();
    if let Ok(payload) = serde_json::to_string(&dtos) {
        if let Ok(mut slot) = registry.dev_modules.lock() {
            *slot = Some(CachedScans { at: std::time::Instant::now(), payload });
        }
    }
    Ok(dtos)
}

#[tauri::command]
async fn clean_dev_modules(registry: State<'_, AppRegistry>, paths: Vec<String>) -> Result<Vec<String>, String> {
    validate_paths(&paths)?;
    use greek_core::dev_modules::DevModulesScanner;
    let scanner = DevModulesScanner::new();
    let pbs: Vec<std::path::PathBuf> = paths.into_iter().map(std::path::PathBuf::from).collect();
    let done = scanner.delete_modules(pbs).await.map_err(|e| e.to_string())?;
    if let Ok(mut slot) = registry.dev_modules.lock() {
        *slot = None;
    }
    Ok(done)
}

#[tauri::command]
async fn clean_all_dev_modules(registry: State<'_, AppRegistry>) -> Result<Vec<String>, String> {
    use greek_core::dev_modules::DevModulesScanner;
    let scanner = DevModulesScanner::new();
    let entries = scanner.scan_all().await.map_err(|e| e.to_string())?;
    let paths: Vec<std::path::PathBuf> = entries.into_iter().map(|e| e.path).collect();
    let done = scanner.delete_modules(paths).await.map_err(|e| e.to_string())?;
    if let Ok(mut slot) = registry.dev_modules.lock() {
        *slot = None;
    }
    Ok(done)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppRegistry::new())
        .invoke_handler(tauri::generate_handler![scan_applications, get_app_details, get_system_stats, analyze_leftovers, clean_leftover_artifacts, uninstall_applications, get_app_icon, get_app_resources, get_app_resource, scan_videos, delete_videos, scan_dev_modules, clean_dev_modules, clean_all_dev_modules])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
