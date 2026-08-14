// Scanner module for discovering installed applications

use greek_common::{
    AppScanner, InstalledApp, InstallSource, Result, RegistryHive, GreekError, ScanError,
};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing;
use rayon::prelude::*;

/// Base scanner implementation with common functionality
pub struct BaseScanner {
    scanner_id: &'static str,
    scanner_name: String,
    requires_elevation: bool,
}

impl BaseScanner {
    pub fn new(scanner_id: &'static str, scanner_name: String, requires_elevation: bool) -> Self {
        Self {
            scanner_id,
            scanner_name,
            requires_elevation,
        }
    }
}

#[async_trait]
impl AppScanner for BaseScanner {
    fn scanner_id(&self) -> &'static str {
        self.scanner_id
    }

    fn scanner_name(&self) -> String {
        self.scanner_name.clone()
    }

    async fn scan(&self) -> Result<Vec<InstalledApp>> {
        Ok(Vec::new())
    }

    fn requires_elevation(&self) -> bool {
        self.requires_elevation
    }
}

/// Scanner manager that coordinates multiple scanners
pub struct ScannerManager {
    scanners: Vec<Box<dyn AppScanner>>,
}

impl ScannerManager {
    pub fn new() -> Self {
        let mut manager = Self {
            scanners: Vec::new(),
        };
        
        // Register platform-specific scanners
        #[cfg(target_os = "windows")]
        {
            tracing::info!("Registering Windows Registry scanner");
            manager.scanners.push(Box::new(greek_windows::WindowsRegistryScanner::new()));
            
            tracing::info!("Registering Windows Store scanner");
            manager.scanners.push(Box::new(greek_windows::WindowsStoreScanner::new()));
        }
        
        manager
    }

    pub fn register_scanner(&mut self, scanner: Box<dyn AppScanner>) {
        tracing::info!("Registering scanner: {}", scanner.scanner_name());
        self.scanners.push(scanner);
    }

    pub fn scanner_count(&self) -> usize {
        self.scanners.len()
    }

    pub fn scanner_names(&self) -> Vec<String> {
        self.scanners.iter().map(|s| s.scanner_name()).collect()
    }

    pub async fn scan_all(&self) -> Result<Vec<InstalledApp>> {
        let mut all_apps = Vec::new();

        tracing::info!("Starting full scan with {} scanners", self.scanners.len());

        for scanner in &self.scanners {
            let scanner_name = scanner.scanner_name();
            tracing::info!("Running scanner: {}", scanner_name);
            
            match scanner.scan().await {
                Ok(apps) => {
                    tracing::info!("Scanner '{}' found {} apps", scanner_name, apps.len());
                    all_apps.extend(apps);
                }
                Err(e) => {
                    tracing::error!("Scanner '{}' failed: {}", scanner_name, e);
                }
            }
        }

        // Deduplicate apps
        all_apps = self.deduplicate_apps(all_apps);
        
        tracing::info!("Total unique apps found: {}", all_apps.len());
        Ok(all_apps)
    }

    pub async fn scan_by_source(&self, _source_type: InstallSource) -> Result<Vec<InstalledApp>> {
        let all_apps = self.scan_all().await?;
        Ok(all_apps)
    }

    fn deduplicate_apps(&self, apps: Vec<InstalledApp>) -> Vec<InstalledApp> {
        use std::collections::HashMap;
        
        let mut unique_apps: HashMap<String, InstalledApp> = HashMap::new();
        
        for app in apps {
            let key = format!("{}-{}", app.name, app.version.as_ref().unwrap_or(&String::new()));
            
            if let Some(existing) = unique_apps.get(&key) {
                if self.is_more_complete(&app, existing) {
                    unique_apps.insert(key, app);
                }
            } else {
                unique_apps.insert(key, app);
            }
        }
        
        unique_apps.into_values().collect()
    }

    fn is_more_complete(&self, app1: &InstalledApp, app2: &InstalledApp) -> bool {
        self.completeness_score(app1) > self.completeness_score(app2)
    }

    fn completeness_score(&self, app: &InstalledApp) -> i32 {
        let mut score = 0;
        if app.publisher.is_some() { score += 1; }
        if app.version.is_some() { score += 1; }
        if app.install_date.is_some() { score += 1; }
        if app.install_location.is_some() { score += 1; }
        if app.uninstall_string.is_some() { score += 1; }
        if app.size_bytes.is_some() { score += 1; }
        if !app.registry_keys.is_empty() { score += 1; }
        score
    }

    /// Parallel scan using rayon for CPU-bound operations
    pub fn scan_parallel(&self, directories: &[PathBuf]) -> Vec<InstalledApp> {
        let all_apps = Mutex::new(Vec::new());
        
        directories.par_iter().for_each(|dir| {
            if let Ok(mut apps) = self.scan_directory_parallel(dir) {
                all_apps.lock().unwrap().append(&mut apps);
            }
        });
        
        let apps = all_apps.into_inner().unwrap();
        self.deduplicate_apps(apps)
    }

    fn scan_directory_parallel(&self, directory: &PathBuf) -> Result<Vec<InstalledApp>> {
        let apps = Mutex::new(Vec::new());
        
        if !directory.exists() {
            return Ok(Vec::new());
        }
        
        let entries: Vec<_> = std::fs::read_dir(directory)
            .map_err(|e| GreekError::ScanError(ScanError::FileSystemScanFailed(e.to_string()).to_string()))?
            .filter_map(|e| e.ok())
            .collect();
        
        entries.par_iter().for_each(|entry| {
            let path = entry.path();
            if path.is_dir() {
                if let Some(app) = self.detect_portable_app_parallel(&path) {
                    apps.lock().unwrap().push(app);
                }
            }
        });
        
        Ok(apps.into_inner().unwrap())
    }

    fn detect_portable_app_parallel(&self, path: &PathBuf) -> Option<InstalledApp> {
        let dir_name = path.file_name()?.to_str()?;
        
        let entries: Vec<_> = std::fs::read_dir(path).ok()?
            .filter_map(|e| e.ok())
            .collect();
        
        entries.par_iter().find_map_any(|entry| {
            let file_path = entry.path();
            
            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    if ext == "exe" {
                        let exe_name = file_path.file_stem()?.to_str()?;
                        
                        if exe_name.to_lowercase() == dir_name.to_lowercase() {
                            let mut app = InstalledApp::new(
                                dir_name.to_string(),
                                InstallSource::Portable {
                                    detected_path: path.clone(),
                                    confidence: 0.8,
                                },
                            );
                            
                            app.install_location = Some(path.clone());
                            return Some(app);
                        }
                    }
                }
            }
            
            None
        })
    }

    pub fn scan_batch_parallel(&self, directory_trees: Vec<PathBuf>, max_depth: usize) -> Vec<InstalledApp> {
        let all_apps = Mutex::new(Vec::new());
        
        directory_trees.par_iter().for_each(|tree| {
            if let Ok(mut apps) = self.scan_directory_tree_parallel(tree, max_depth) {
                all_apps.lock().unwrap().append(&mut apps);
            }
        });
        
        let apps = all_apps.into_inner().unwrap();
        self.deduplicate_apps(apps)
    }

    fn scan_directory_tree_parallel(&self, root: &PathBuf, max_depth: usize) -> Result<Vec<InstalledApp>> {
        let apps = Mutex::new(Vec::new());
        
        fn scan_recursive(
            dir: &PathBuf,
            depth: usize,
            max_depth: usize,
            apps: &Mutex<Vec<InstalledApp>>,
            scanner: &ScannerManager,
        ) {
            if depth > max_depth {
                return;
            }
            
            if let Ok(entries) = std::fs::read_dir(dir) {
                let entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                
                entries.par_iter().for_each(|entry| {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(app) = scanner.detect_portable_app_parallel(&path) {
                            apps.lock().unwrap().push(app);
                        }
                        scan_recursive(&path, depth + 1, max_depth, apps, scanner);
                    }
                });
            }
        }
        
        scan_recursive(root, 0, max_depth, &apps, self);
        
        Ok(apps.into_inner().unwrap())
    }
}

impl Default for ScannerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Portable app scanner
pub struct PortableAppScanner {
    base: BaseScanner,
    scan_directories: Vec<PathBuf>,
}

impl PortableAppScanner {
    pub fn new(scan_directories: Vec<PathBuf>) -> Self {
        Self {
            base: BaseScanner::new(
                "portable-apps",
                "Portable Application Scanner".to_string(),
                false,
            ),
            scan_directories,
        }
    }
}

#[async_trait]
impl AppScanner for PortableAppScanner {
    fn scanner_id(&self) -> &'static str {
        self.base.scanner_id()
    }

    fn scanner_name(&self) -> String {
        self.base.scanner_name()
    }

    async fn scan(&self) -> Result<Vec<InstalledApp>> {
        let mut apps = Vec::new();
        
        for directory in &self.scan_directories {
            if !directory.exists() {
                tracing::warn!("Portable scan directory does not exist: {:?}", directory);
                continue;
            }
            
            let discovered = self.scan_directory(directory).await?;
            apps.extend(discovered);
        }
        
        Ok(apps)
    }

    fn requires_elevation(&self) -> bool {
        self.base.requires_elevation()
    }
}

impl PortableAppScanner {
    async fn scan_directory(&self, directory: &PathBuf) -> Result<Vec<InstalledApp>> {
        let mut apps = Vec::new();
        
        let entries = std::fs::read_dir(directory)
            .map_err(|e| GreekError::ScanError(ScanError::FileSystemScanFailed(e.to_string()).to_string()))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| GreekError::ScanError(ScanError::FileSystemScanFailed(e.to_string()).to_string()))?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(app) = self.detect_portable_app(&path).await? {
                    apps.push(app);
                }
            }
        }
        
        Ok(apps)
    }

    async fn detect_portable_app(&self, path: &PathBuf) -> Result<Option<InstalledApp>> {
        let dir_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        for entry in std::fs::read_dir(path)
            .map_err(|e| GreekError::ScanError(ScanError::FileSystemScanFailed(e.to_string()).to_string()))?
        {
            let entry = entry.map_err(|e| GreekError::ScanError(ScanError::FileSystemScanFailed(e.to_string()).to_string()))?;
            let file_path = entry.path();
            
            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    if ext == "exe" {
                        let exe_name = file_path.file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");
                        
                        if exe_name.to_lowercase() == dir_name.to_lowercase() {
                            let mut app = InstalledApp::new(
                                dir_name.to_string(),
                                InstallSource::Portable {
                                    detected_path: path.clone(),
                                    confidence: 0.8,
                                },
                            );
                            
                            app.install_location = Some(path.clone());
                            return Ok(Some(app));
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
}
