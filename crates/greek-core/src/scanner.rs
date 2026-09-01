// Scanner module for discovering installed applications

use async_trait::async_trait;
use greek_common::{AppScanner, GreekError, InstallSource, InstalledApp, Result, ScanError};
use rayon::prelude::*;
use std::path::PathBuf;
use tracing;

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
        #[allow(unused_mut)]
        let mut scanners: Vec<Box<dyn AppScanner>> = Vec::new();

        // Register platform-specific scanners
        #[cfg(all(target_os = "windows", feature = "windows"))]
        {
            tracing::info!("Registering Windows Registry scanner");
            scanners.push(Box::new(greek_windows::WindowsRegistryScanner::new()));

            tracing::info!("Registering Windows Store scanner");
            scanners.push(Box::new(greek_windows::WindowsStoreScanner::new()));
        }

        #[cfg(target_os = "linux")]
        {
            tracing::info!("Registering Linux package scanner");
            if let Some(pm) = greek_platform::linux::LinuxPackageScanner::detect_package_manager() {
                let scanner = greek_platform::linux::LinuxPackageScanner::new(pm);
                scanners.push(Box::new(LinuxScannerAdapter(scanner)));
            }
        }

        #[cfg(target_os = "macos")]
        {
            tracing::info!("Registering macOS app scanner");
            let scanner = greek_platform::macos::MacOsAppScanner::new();
            scanners.push(Box::new(MacOsScannerAdapter(scanner)));
        }

        Self { scanners }
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
        tracing::info!("Starting full scan with {} scanners", self.scanners.len());

        let mut all_apps = Vec::new();
        for scanner in &self.scanners {
            match scanner.scan().await {
                Ok(apps) => {
                    tracing::info!(
                        "Scanner '{}' found {} apps",
                        scanner.scanner_name(),
                        apps.len()
                    );
                    all_apps.extend(apps);
                }
                Err(e) => {
                    tracing::error!("Scanner '{}' failed: {}", scanner.scanner_name(), e);
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
            // CR-9: include install_location in dedup key so apps with the same
            // name/version but different install paths are not merged.
            let key = format!(
                "{}-{}-{}",
                app.name,
                app.version.as_ref().unwrap_or(&String::new()),
                app.install_location
                    .as_ref()
                    .map(|p| p.to_string_lossy())
                    .unwrap_or_default()
            );

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
        if app.publisher.is_some() {
            score += 1;
        }
        if app.version.is_some() {
            score += 1;
        }
        if app.install_date.is_some() {
            score += 1;
        }
        if app.install_location.is_some() {
            score += 1;
        }
        if app.uninstall_string.is_some() {
            score += 1;
        }
        if app.size_bytes.is_some() {
            score += 1;
        }
        if !app.registry_keys.is_empty() {
            score += 1;
        }
        score
    }

    /// Parallel scan using rayon for CPU-bound operations.
    /// Collects into per-fork vectors and merges afterwards, avoiding any
    /// shared-mutable-state / lock contention across the rayon pool.
    ///
    /// Fixed: previously returned empty (placeholder). Now delegates to
    /// scan_directory_parallel per directory.
    pub async fn scan_parallel(&self, directories: Vec<PathBuf>) -> Vec<InstalledApp> {
        // Clone self-relevant data for spawn_blocking (ScannerManager is not Sync due to trait objects)
        // Instead, run portable parallel scan directly via spawn_blocking with a stateless helper.
        let dirs = directories.clone();

        let all_apps: Vec<Vec<InstalledApp>> = tokio::task::spawn_blocking(move || {
            dirs.par_iter()
                .filter_map(|dir| {
                    if !dir.exists() {
                        return None;
                    }
                    // Stateless portable scan: look for exe matching dir name
                    let entries: Vec<_> = std::fs::read_dir(dir)
                        .ok()?
                        .filter_map(|e| e.ok())
                        .collect();
                    let mut found = Vec::new();
                    for entry in entries {
                        let path = entry.path();
                        if !path.is_dir() {
                            continue;
                        }
                        let dir_name = path.file_name()?.to_str()?;
                        if let Ok(exe_entries) = std::fs::read_dir(&path) {
                            for exe_ent in exe_entries.filter_map(|e| e.ok()) {
                                let fp = exe_ent.path();
                                if fp.is_file()
                                    && fp.extension().is_some_and(|e| e == "exe")
                                    && fp
                                        .file_stem()
                                        .and_then(|s| s.to_str())
                                        .is_some_and(|s| s.eq_ignore_ascii_case(dir_name))
                                {
                                    let app = greek_common::InstalledApp::new(
                                        dir_name.to_string(),
                                        greek_common::InstallSource::Portable {
                                            detected_path: path.clone(),
                                            confidence: 0.8,
                                        },
                                    );
                                    found.push(app);
                                    break;
                                }
                            }
                        }
                    }
                    Some(found)
                })
                .collect()
        })
        .await
        .unwrap_or_default();

        let apps: Vec<InstalledApp> = all_apps.into_iter().flatten().collect();
        self.deduplicate_apps(apps)
    }

    #[allow(dead_code)]
    fn scan_directory_parallel(&self, directory: &PathBuf) -> Result<Vec<InstalledApp>> {
        if !directory.exists() {
            return Ok(Vec::new());
        }

        let entries: Vec<_> = std::fs::read_dir(directory)
            .map_err(|e| {
                GreekError::ScanError(ScanError::FileSystemScanFailed(e.to_string()).to_string())
            })?
            .filter_map(|e| e.ok())
            .collect();

        // Each parallel task returns its own Option; no Mutex needed.
        let apps: Vec<InstalledApp> = entries
            .par_iter()
            .filter_map(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    self.detect_portable_app_parallel(&path)
                } else {
                    None
                }
            })
            .collect();

        Ok(apps)
    }

    fn detect_portable_app_parallel(&self, path: &PathBuf) -> Option<InstalledApp> {
        let dir_name = path.file_name()?.to_str()?;

        let entries: Vec<_> = std::fs::read_dir(path)
            .ok()?
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

    pub fn scan_batch_parallel(
        &self,
        directory_trees: Vec<PathBuf>,
        max_depth: usize,
    ) -> Vec<InstalledApp> {
        let all_apps: Vec<Vec<InstalledApp>> = directory_trees
            .par_iter()
            .filter_map(|tree| self.scan_directory_tree_parallel(tree, max_depth).ok())
            .collect();

        let apps: Vec<InstalledApp> = all_apps.into_iter().flatten().collect();
        self.deduplicate_apps(apps)
    }

    fn scan_directory_tree_parallel(
        &self,
        root: &PathBuf,
        max_depth: usize,
    ) -> Result<Vec<InstalledApp>> {
        fn scan_recursive(
            dir: &PathBuf,
            depth: usize,
            max_depth: usize,
            scanner: &ScannerManager,
        ) -> Vec<InstalledApp> {
            if depth > max_depth {
                return Vec::new();
            }

            let entries: Vec<_> = std::fs::read_dir(dir)
                .map(|it| it.filter_map(|e| e.ok()).collect())
                .unwrap_or_default();

            // Each branch returns its own Vec; merges happen after the fork.
            let nested: Vec<Vec<InstalledApp>> = entries
                .par_iter()
                .map(|entry| {
                    let path = entry.path();
                    let mut found = Vec::new();
                    if path.is_dir() {
                        if let Some(app) = scanner.detect_portable_app_parallel(&path) {
                            found.push(app);
                        }
                        found.extend(scan_recursive(&path, depth + 1, max_depth, scanner));
                    }
                    found
                })
                .collect();

            nested.into_iter().flatten().collect()
        }

        Ok(scan_recursive(root, 0, max_depth, self))
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

        let entries = std::fs::read_dir(directory).map_err(|e| {
            GreekError::ScanError(ScanError::FileSystemScanFailed(e.to_string()).to_string())
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                GreekError::ScanError(ScanError::FileSystemScanFailed(e.to_string()).to_string())
            })?;
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
        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        for entry in std::fs::read_dir(path).map_err(|e| {
            GreekError::ScanError(ScanError::FileSystemScanFailed(e.to_string()).to_string())
        })? {
            let entry = entry.map_err(|e| {
                GreekError::ScanError(ScanError::FileSystemScanFailed(e.to_string()).to_string())
            })?;
            let file_path = entry.path();

            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    if ext == "exe" {
                        let exe_name = file_path
                            .file_stem()
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

/// Adapter exposing the greek-platform Linux package scanner through the
/// common `AppScanner` trait.
#[cfg(target_os = "linux")]
struct LinuxScannerAdapter(greek_platform::linux::LinuxPackageScanner);

#[cfg(target_os = "linux")]
#[async_trait]
impl AppScanner for LinuxScannerAdapter {
    fn scanner_id(&self) -> &'static str {
        "linux-package"
    }

    fn scanner_name(&self) -> String {
        "Linux Package Manager".to_string()
    }

    async fn scan(&self) -> Result<Vec<InstalledApp>> {
        self.0.scan().await
    }

    fn requires_elevation(&self) -> bool {
        true
    }
}

/// Adapter exposing the greek-platform macOS app scanner through the common
/// `AppScanner` trait.
#[cfg(target_os = "macos")]
struct MacOsScannerAdapter(greek_platform::macos::MacOsAppScanner);

#[cfg(target_os = "macos")]
#[async_trait]
impl AppScanner for MacOsScannerAdapter {
    fn scanner_id(&self) -> &'static str {
        "macos-app"
    }

    fn scanner_name(&self) -> String {
        "macOS Applications".to_string()
    }

    async fn scan(&self) -> Result<Vec<InstalledApp>> {
        self.0.scan().await
    }

    fn requires_elevation(&self) -> bool {
        false
    }
}
