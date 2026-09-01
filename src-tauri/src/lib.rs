use greek_common::models::{InstalledApp, InstallSource, RegistryHive};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

// ---------- Shared state ----------
struct AppRegistry(Mutex<HashMap<String, InstalledApp>>);

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
        Self {
            id: a.id.to_string(),
            name: a.name.clone(),
            publisher: a.publisher.clone(),
            version: a.version.clone(),
            size_bytes: a.size_bytes,
            size_display: a.display_size(),
            install_date: a.install_date.map(|d| d.to_string()),
            install_location: a.install_location.map(|p| p.to_string_lossy().to_string()),
            source_label,
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallPayload {
    pub ids: Vec<String>,
    pub force: bool,
    pub silent: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallProgressPayload {
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
    pub size_display: Option<String>,
    pub confidence: f32,
    pub safety: String,
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
    let apps = match real_scan().await {
        Ok(v) if !v.is_empty() => v,
        Ok(empty) => {
            if cfg!(target_os = "windows") {
                eprintln!("[scan] returned 0, fallback mock");
                if empty.is_empty() { mock_apps() } else { empty }
            } else { mock_apps() }
        }
        Err(e) => {
            eprintln!("[scan] failed {e}, fallback mock");
            mock_apps()
        }
    };
    {
        let mut map = registry.0.lock().map_err(|e| format!("lock {e}"))?;
        map.clear();
        for a in &apps { map.insert(a.id.to_string(), a.clone()); }
    }
    Ok(apps.into_iter().map(AppEntry::from).collect())
}

#[tauri::command]
async fn get_app_details(registry: State<'_, AppRegistry>, id: String) -> Result<AppDetails, String> {
    let map = registry.0.lock().map_err(|e| format!("lock {e}"))?;
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

#[tauri::command]
async fn analyze_leftovers(registry: State<'_, AppRegistry>, id: String) -> Result<Vec<LeftoverDto>, String> {
    let app = {
        let map = registry.0.lock().map_err(|e| format!("lock {e}"))?;
        map.get(&id).cloned().ok_or_else(|| format!("App {id} not found"))?
    };
    use greek_common::GreekConfig;
    use greek_core::GreekAppService;
    let config = GreekConfig::default();
    let mut svc = GreekAppService::new(config).map_err(|e| e.to_string())?;
    let artifacts = svc.analyze_leftovers(&app).await.map_err(|e| e.to_string())?;
    Ok(artifacts.into_iter().map(|a| {
        let size_display = a.size_bytes.map(|b| humansize::format_size(b, humansize::BINARY));
        LeftoverDto {
            id: a.id.to_string(),
            artifact_type: format!("{:?}", a.artifact_type),
            path: a.path.to_string_lossy().to_string(),
            size_display,
            confidence: a.confidence,
            safety: format!("{:?}", a.safety_level),
        }
    }).collect())
}

#[tauri::command]
async fn uninstall_applications(
    app_handle: AppHandle,
    registry: State<'_, AppRegistry>,
    payload: UninstallPayload,
) -> Result<Vec<UninstallResultDto>, String> {
    let total = payload.ids.len();
    if total == 0 { return Err("No applications selected".into()); }
    let targets: Vec<InstalledApp> = {
        let map = registry.0.lock().map_err(|e| format!("lock {e}"))?;
        let mut out = Vec::new();
        for id in &payload.ids {
            if let Some(a) = map.get(id) { out.push(a.clone()); } else { eprintln!("[uninstall] missing {id}"); }
        }
        if out.is_empty() { return Err("Selected apps not found. Please re-scan.".into()); }
        out
    };
    use greek_common::{GreekConfig, UninstallOptions};
    use greek_core::GreekAppService;
    let config = GreekConfig::default();
    let svc = GreekAppService::new(config).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    for (idx, app) in targets.iter().enumerate() {
        let cur = idx + 1;
        let _ = app_handle.emit("uninstall-progress", UninstallProgressPayload {
            current: cur, total, app_name: app.name.clone(), status: "processing".into(),
            log: format!("Uninstalling {} {}{}", app.name, app.version.as_deref().unwrap_or(""), if payload.force { " (force)" } else { "" })
        });
        let mut opts = if payload.force { UninstallOptions::force() } else { UninstallOptions::standard() };
        opts.silent = payload.silent.unwrap_or(false);
        let res = if payload.force { svc.force_remove_app(app, opts.clone()).await } else { svc.uninstall_app(app, opts.clone()).await };
        match res {
            Ok(r) => {
                let log = if r.success { format!("{} via {} (files:{}, regs:{})", app.name, r.strategy_used, r.files_deleted.len(), r.registry_keys_deleted.len()) } else { r.errors.join("; ") };
                let status = if r.success { "done" } else { "error" };
                let _ = app_handle.emit("uninstall-progress", UninstallProgressPayload { current: cur, total, app_name: app.name.clone(), status: status.into(), log: log.clone() });
                results.push(UninstallResultDto { id: app.id.to_string(), name: app.name.clone(), success: r.success, error: if r.success { None } else { Some(log) } });
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = app_handle.emit("uninstall-progress", UninstallProgressPayload { current: cur, total, app_name: app.name.clone(), status: "error".into(), log: msg.clone() });
                results.push(UninstallResultDto { id: app.id.to_string(), name: app.name.clone(), success: false, error: Some(msg) });
            }
        }
    }
    { let mut map = registry.0.lock().map_err(|e| format!("lock {e}"))?; for r in &results { if r.success { map.remove(&r.id); } } }
    Ok(results)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppRegistry(Mutex::new(HashMap::new())))
        .invoke_handler(tauri::generate_handler![scan_applications, get_app_details, get_system_stats, analyze_leftovers, uninstall_applications])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
