// Core application service coordinating all uninstallation operations

use crate::config::ConfigManager;
use crate::leftover::LeftoverAnalyzerManager;
use crate::scanner::ScannerManager;
use crate::uninstaller::{ForceRemoveStrategy, UninstallerManager};
use greek_common::UninstallStrategy;
use greek_common::{
    AppEvent, ArtifactType, BatchQueue, GreekConfig, GreekError, InstalledApp, LeftoverArtifact,
    Result, SafetyLevel, UninstallOptions, UninstallResult,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

#[cfg(all(target_os = "windows", feature = "windows"))]
use greek_windows::icon::IconExtractor;

/// One leftover item nominated for deletion by path (IPC-friendly: no
/// cross-call UUID cache required).
#[derive(Debug, Clone)]
pub struct CleanLeftoverRequest {
    pub path: String,
    pub artifact_type: String,
    pub safety: String,
}

/// Per-item outcome of a [`GreekAppService::clean_leftover_paths`] call.
#[derive(Debug, Clone)]
pub struct CleanLeftoverOutcome {
    pub path: String,
    pub deleted: bool,
    pub reason: String,
}

impl CleanLeftoverOutcome {
    fn deleted(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self { path: path.into(), deleted: true, reason: reason.into() }
    }
    fn skipped(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self { path: path.into(), deleted: false, reason: reason.into() }
    }
}

/// Main service that coordinates all uninstaller operations
pub struct GreekAppService {
    config: GreekConfig,
    scanner_manager: ScannerManager,
    uninstaller_manager: UninstallerManager,
    leftover_analyzer_manager: LeftoverAnalyzerManager,
    event_sender: broadcast::Sender<AppEvent>,
    /// Cache of leftover artifacts from the most recent analysis, keyed by app id.
    /// Used by `clean_leftovers` to look up artifacts by UUID without re-scanning.
    artifact_cache: HashMap<uuid::Uuid, LeftoverArtifact>,
    /// MED-1: cached scan results with timestamp for TTL-based cache.
    app_cache: Option<(Vec<InstalledApp>, Instant)>,
    /// TTL for the app scan cache (default 30 seconds).
    cache_ttl: Duration,
}

impl GreekAppService {
    pub fn new(config: GreekConfig) -> Result<Self> {
        let (event_sender, _) = broadcast::channel(100);

        let mut scanner_manager = ScannerManager::new();
        let uninstaller_manager = UninstallerManager::new();
        let mut leftover_analyzer_manager = LeftoverAnalyzerManager::new();

        // Register portable app scanner if dirs configured
        if !config.scanner.scan_portable_dirs.is_empty() {
            use crate::scanner::PortableAppScanner;
            use std::path::PathBuf;

            let scan_dirs: Vec<PathBuf> = config
                .scanner
                .scan_portable_dirs
                .iter()
                .map(PathBuf::from)
                .collect();

            scanner_manager.register_scanner(Box::new(PortableAppScanner::new(scan_dirs)));
        }

        // Register filesystem leftover analyzer - comprehensive: all drives, Program Files, Users/AppData, Windows (shallow)
        {
            use crate::leftover::{build_comprehensive_scan_roots, FileSystemLeftoverAnalyzer};

            // Use comprehensive roots (all drives + Program Files + AppData per user + ProgramData + Windows shallow)
            let comprehensive = build_comprehensive_scan_roots();
            tracing::info!("Registering filesystem leftover analyzer with {} roots: {:?}", comprehensive.len(), comprehensive);
            leftover_analyzer_manager
                .register_analyzer(Box::new(FileSystemLeftoverAnalyzer::new(comprehensive)));
        }

        // Register whole-device analyzers: registry, junk/temp, services, tasks, shortcuts, duplicate downloads
        #[cfg(all(target_os = "windows", feature = "windows"))]
        {
            use crate::leftover::{
                DuplicateDownloadAnalyzer, JunkLeftoverAnalyzer, RegistryLeftoverAnalyzer,
                ServiceLeftoverAnalyzer, ShortcutLeftoverAnalyzer, TaskLeftoverAnalyzer,
            };
            leftover_analyzer_manager.register_analyzer(Box::new(RegistryLeftoverAnalyzer::new()));
            leftover_analyzer_manager.register_analyzer(Box::new(JunkLeftoverAnalyzer::new()));
            leftover_analyzer_manager.register_analyzer(Box::new(ServiceLeftoverAnalyzer::new()));
            leftover_analyzer_manager.register_analyzer(Box::new(TaskLeftoverAnalyzer::new()));
            leftover_analyzer_manager.register_analyzer(Box::new(ShortcutLeftoverAnalyzer::new()));
            leftover_analyzer_manager.register_analyzer(Box::new(DuplicateDownloadAnalyzer::new()));
            tracing::info!("Registered registry/junk/service/task/shortcut/duplicate leftover analyzers");
        }

        tracing::info!(
            "GreekAppService initialized with {} scanners",
            scanner_manager.scanner_count()
        );
        for name in scanner_manager.scanner_names() {
            tracing::info!("  - Scanner: {}", name);
        }

        let cache_ttl = Duration::from_secs(30);
        let service = Self {
            config,
            scanner_manager,
            uninstaller_manager,
            leftover_analyzer_manager,
            event_sender,
            artifact_cache: HashMap::new(),
            app_cache: None,
            cache_ttl,
        };

        Ok(service)
    }

    pub fn from_config_manager(config_manager: &ConfigManager) -> Result<Self> {
        let config = config_manager.load_config()?;
        Self::new(config)
    }

    /// Register a custom scanner
    pub fn register_scanner(&mut self, scanner: Box<dyn greek_common::AppScanner>) {
        self.scanner_manager.register_scanner(scanner);
    }

    /// Register a custom uninstall strategy
    pub fn register_strategy(&mut self, strategy: Box<dyn greek_common::UninstallStrategy>) {
        self.uninstaller_manager.register_strategy(strategy);
    }

    /// Register a custom leftover analyzer
    pub fn register_analyzer(&mut self, analyzer: Box<dyn greek_common::LeftoverAnalyzer>) {
        self.leftover_analyzer_manager.register_analyzer(analyzer);
    }

    /// Scan all sources for installed applications.
    ///
    /// MED-1: results are cached for `cache_ttl` to avoid redundant full
    /// system scans when multiple operations occur in quick succession.
    pub async fn scan_all_apps(&mut self) -> Result<Vec<InstalledApp>> {
        // Check cache first
        if let Some((ref cached, timestamp)) = self.app_cache {
            if timestamp.elapsed() < self.cache_ttl {
                tracing::debug!("Returning cached app list ({} apps)", cached.len());
                return Ok(cached.clone());
            }
        }

        tracing::info!("Starting full system scan");

        let _ = self.event_sender.send(AppEvent::ScanStarted {
            scanner_id: "all".to_string(),
        });

        let mut apps = self
            .scanner_manager
            .scan_all()
            .await
            .map_err(|e| GreekError::ScanError(e.to_string()))?;

        // Enrich apps: extract real icons, compute dominant colors, fill missing sizes
        let mut apps = tokio::task::spawn_blocking(move || {
            #[cfg(all(target_os = "windows", feature = "windows"))]
            {
                let extractor = IconExtractor::new();
                extractor.enrich_apps(&mut apps);
            }
            #[cfg(not(all(target_os = "windows", feature = "windows")))]
            {
                let _ = &mut apps;
            }
            apps
        })
        .await
        .map_err(|e| GreekError::ScanError(format!("Task join error: {}", e)))?;

        // Filter OS-critical / system-bundled apps: only show safely removable apps.
        // This is the user-requested default — apps whose removal could brick the OS,
        // drivers, or Windows itself are hidden. The registry/store scanners already
        // mark most of these via `is_system_component`, but this is the final safety net
        // (covers portable / package-manager sources and name-heuristic matches).
        let total_before_filter = apps.len();
        apps.retain(|a| a.is_safe_to_show());
        let filtered = total_before_filter.saturating_sub(apps.len());
        if filtered > 0 {
            tracing::info!(
                "Filtered {} OS-critical/system apps (showing {} safe-to-remove)",
                filtered,
                apps.len()
            );
        }

        // Hide entries with no supported removal path (stale references,
        // extension listings without backing data): the desktop list only
        // shows applications that can actually be deleted.
        let before_removal_filter = apps.len();
        apps.retain(|a| a.has_removal_path());
        let unremovable = before_removal_filter.saturating_sub(apps.len());
        if unremovable > 0 {
            tracing::info!(
                "Filtered {} apps with no removal path (showing {} deletable)",
                unremovable,
                apps.len()
            );
        }

        let _ = self.event_sender.send(AppEvent::ScanCompleted {
            scanner_id: "all".to_string(),
            count: apps.len(),
        });

        tracing::info!("System scan completed, found {} applications ({} OS-critical filtered, {} without removal path)", apps.len(), filtered, unremovable);

        // MED-1: cache the results
        self.app_cache = Some((apps.clone(), Instant::now()));

        Ok(apps)
    }

    /// Get detailed info about a specific app
    pub async fn get_app_details(&mut self, app_id: uuid::Uuid) -> Result<InstalledApp> {
        let apps = self.scan_all_apps().await?;

        apps.into_iter()
            .find(|app| app.id == app_id)
            .ok_or_else(|| GreekError::NotFound(format!("App with ID {} not found", app_id)))
    }

    /// Uninstall a single application
    pub async fn uninstall_app(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult> {
        // Safety: refuse to uninstall OS-critical apps
        if app.is_os_critical() {
            tracing::error!("Blocked uninstall of OS-critical app: {}", app.name);
            return Err(GreekError::SafetyError(format!(
                "Refusing to uninstall OS-critical application '{}' — removal could break the operating system or drivers.",
                app.name
            )));
        }
        tracing::info!("Starting uninstall for: {}", app.name);

        let _ = self.event_sender.send(AppEvent::UninstallStarted {
            app_id: app.id,
            app_name: app.name.clone(),
        });

        // Create a restore point before uninstalling when requested.
        if options.create_restore_point {
            self.create_restore_point(&format!("REEK uninstall: {}", app.name))
                .await?;
        }

        let result = self
            .uninstaller_manager
            .uninstall(app, options)
            .await
            .map_err(|e| GreekError::UninstallError(e.to_string()))?;

        let _ = self.event_sender.send(AppEvent::UninstallCompleted {
            app_id: app.id,
            result: result.clone(),
        });

        tracing::info!(
            "Uninstall completed for: {} - Success: {}",
            app.name,
            result.success
        );

        Ok(result)
    }

    /// Create a system restore point. On Windows this uses the RestorePointManager;
    /// on other platforms it is a no-op (no System Restore equivalent).
    async fn create_restore_point(&self, description: &str) -> Result<()> {
        #[cfg(all(target_os = "windows", feature = "windows"))]
        {
            tracing::info!("Creating restore point: {}", description);
            let manager = greek_windows::RestorePointManager::new();
            match manager.create_restore_point(description).await {
                Ok(_) => Ok(()),
                // Restore points may be disabled / unavailable; warn but do not
                // block the uninstall.
                Err(e) => {
                    tracing::warn!("Failed to create restore point: {}", e);
                    Ok(())
                }
            }
        }

        #[cfg(not(all(target_os = "windows", feature = "windows")))]
        {
            let _ = description;
            Ok(())
        }
    }

    /// Force remove an application.
    ///
    /// True force: bypasses the vendor uninstaller entirely and deletes the
    /// install directory plus recorded registry keys directly. (The strategy
    /// picker would otherwise select MSI/Standard first whenever an
    /// uninstall string exists, making "force" a misnomer.)
    pub async fn force_remove_app(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult> {
        if app.is_os_critical() {
            tracing::error!("Blocked force remove of OS-critical app: {}", app.name);
            return Err(GreekError::SafetyError(format!(
                "Refusing to force-remove OS-critical application '{}'.",
                app.name
            )));
        }
        tracing::warn!("Starting force remove for: {}", app.name);

        let mut force_options = options;
        force_options.force = true;

        if force_options.create_restore_point {
            self.create_restore_point(&format!("REEK force remove: {}", app.name))
                .await?;
        }

        // Store apps live under the protected WindowsApps tree, so file
        // deletion can never remove them: Appx removal IS the force path.
        #[cfg(all(target_os = "windows", feature = "windows"))]
        if matches!(
            &app.source,
            greek_common::InstallSource::WindowsStore { .. }
        ) {
            let strategy = crate::uninstaller::StoreUninstallStrategy::new();
            let result = strategy
                .uninstall(app, force_options)
                .await
                .map_err(|e| GreekError::UninstallError(e.to_string()))?;
            tracing::warn!(
                "Force remove (Store) completed for: {} - Success: {}",
                app.name,
                result.success
            );
            return Ok(result);
        }

        let strategy = ForceRemoveStrategy::new();
        let result = strategy
            .uninstall(app, force_options)
            .await
            .map_err(|e| GreekError::UninstallError(e.to_string()))?;

        tracing::warn!(
            "Force remove completed for: {} - Success: {}",
            app.name,
            result.success
        );

        Ok(result)
    }

    /// Analyze leftovers for an app.
    ///
    /// Results are cached internally so `clean_leftovers` can look them up
    /// by UUID without re-scanning.
    pub async fn analyze_leftovers(&mut self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        tracing::info!("Analyzing leftovers for: {}", app.name);

        let _ = self
            .event_sender
            .send(AppEvent::LeftoverScanStarted { app_id: app.id });

        let artifacts = self
            .leftover_analyzer_manager
            .analyze_app(app)
            .await
            .map_err(|e| GreekError::AnalysisError(e.to_string()))?;

        // Cache artifacts so clean_leftovers can look them up by UUID
        for artifact in &artifacts {
            self.artifact_cache.insert(artifact.id, artifact.clone());
            let _ = self.event_sender.send(AppEvent::LeftoverFound {
                app_id: app.id,
                artifact: artifact.clone(),
            });
        }

        tracing::info!(
            "Leftover analysis completed for: {}, found {} artifacts",
            app.name,
            artifacts.len()
        );

        Ok(artifacts)
    }

    /// Clean up leftover artifacts by their UUIDs.
    ///
    /// Security: checks every artifact path against `PROTECTED_PATHS` and
    /// respects `SafetyLevel` — only `Safe` artifacts are deleted unless
    /// `options.force` is true.
    pub async fn clean_leftovers(
        &self,
        artifact_ids: Vec<uuid::Uuid>,
        options: UninstallOptions,
    ) -> Result<()> {
        tracing::info!("Cleaning up {} leftover artifacts", artifact_ids.len());

        let protected = greek_common::PROTECTED_PATHS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        for artifact_id in &artifact_ids {
            let artifact = self.artifact_cache.get(artifact_id).ok_or_else(|| {
                GreekError::NotFound(format!(
                    "Artifact {} not found in cache; run analyze_leftovers first",
                    artifact_id
                ))
            })?;

            // Refuse to delete protected paths
            if greek_common::is_protected_path(&artifact.path, &protected) {
                tracing::error!(
                    "Blocked deletion of protected path: {}",
                    artifact.path.display()
                );
                continue;
            }

            // Only delete Safe artifacts unless force mode
            if artifact.safety_level != SafetyLevel::Safe && !options.force {
                tracing::warn!(
                    "Skipping non-safe artifact ({:?}): {}",
                    artifact.safety_level,
                    artifact.path.display()
                );
                continue;
            }

            // Delete based on artifact type
            match artifact.artifact_type {
                ArtifactType::File => {
                    if artifact.path.exists() {
                        crate::utils::delete_file(&artifact.path)?;
                        tracing::info!("Deleted leftover file: {}", artifact.path.display());
                    }
                }
                ArtifactType::Directory => {
                    if artifact.path.exists() {
                        crate::utils::delete_directory(&artifact.path)?;
                        tracing::info!("Deleted leftover directory: {}", artifact.path.display());
                    }
                }
                ArtifactType::RegistryKey => {
                    crate::utils::delete_registry_key(&artifact.path.to_string_lossy())?;
                    tracing::info!("Deleted leftover registry key: {}", artifact.path.display());
                }
                other => {
                    tracing::warn!(
                        "Unsupported artifact type {:?} for: {}",
                        other,
                        artifact.path.display()
                    );
                }
            }
        }

        tracing::info!("Leftover cleanup completed");
        Ok(())
    }

    /// Delete leftover artifacts by explicit path.
    ///
    /// Unlike [`clean_leftovers`] (UUIDs resolved through this instance's
    /// cache), this takes caller-supplied paths so it works across IPC calls,
    /// where every command builds a fresh service. Each path is re-validated:
    /// protected filesystem/registry paths are always refused; when
    /// `only_safe` is true, only `Safe` items are deleted (auto-clean after
    /// uninstall). Explicit user-confirmed cleans pass `only_safe = false`.
    pub async fn clean_leftover_paths(
        &self,
        items: Vec<CleanLeftoverRequest>,
        only_safe: bool,
    ) -> Vec<CleanLeftoverOutcome> {
        let protected: Vec<String> = greek_common::PROTECTED_PATHS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let path_str = item.path.trim().to_string();
            if path_str.is_empty() {
                out.push(CleanLeftoverOutcome::skipped(item.path, "empty path"));
                continue;
            }
            let is_safe = item.safety.eq_ignore_ascii_case("safe");
            if only_safe && !is_safe {
                out.push(CleanLeftoverOutcome::skipped(
                    path_str,
                    "requires explicit confirmation",
                ));
                continue;
            }
            // Pseudo-paths for services never touch the filesystem guard.
            let is_service = item
                .artifact_type
                .eq_ignore_ascii_case("service")
                || path_str.to_lowercase().starts_with("service\\");
            if !is_service {
                let probe = std::path::PathBuf::from(&path_str);
                // Registry pseudo-paths are covered by the registry guard
                // inside the delete helpers; only gate real filesystem paths.
                let looks_registry = {
                    let l = path_str.to_lowercase();
                    l.starts_with("hklm\\") || l.starts_with("hkcu\\")
                };
                if !looks_registry && greek_common::is_protected_path(&probe, &protected) {
                    out.push(CleanLeftoverOutcome::skipped(path_str, "protected path"));
                    continue;
                }
            }
            out.push(self.delete_single_leftover(&item, &path_str).await);
        }
        out
    }

    async fn delete_single_leftover(
        &self,
        item: &CleanLeftoverRequest,
        path_str: &str,
    ) -> CleanLeftoverOutcome {
        let kind = item.artifact_type.to_lowercase();
        let res: Result<()> = match kind.as_str() {
            // Windows service recorded as `Service\<name>`.
            "service" => {
                let name = path_str
                    .strip_prefix("Service\\")
                    .or_else(|| path_str.strip_prefix("service\\"))
                    .unwrap_or(path_str);
                self.delete_orphan_service(name).await
            }
            "scheduledtask" | "scheduled_task" | "task" => {
                crate::task_scheduler::TaskSchedulerScanner::new()
                    .delete_task(path_str)
                    .await
            }
            "registrykey" | "registry_key" => {
                crate::utils::delete_registry_key(path_str)
            }
            "registryvalue" | "registry_value" => {
                // Format from the analyzer: `HKLM\path\to\key\ [ValueName]`.
                match path_str.rsplit_once("\\ [") {
                    Some((key, rest)) => match rest.strip_suffix(']') {
                        Some(value) => crate::utils::delete_registry_value(key, value),
                        None => Err(greek_common::GreekError::RegistryError(
                            "Unparseable registry value path".to_string(),
                        )),
                    },
                    None => Err(greek_common::GreekError::RegistryError(
                        "Unparseable registry value path".to_string(),
                    )),
                }
            }
            // Shell extensions and drivers are never auto-deleted: removing the
            // wrong one breaks Explorer or boot. Report explicitly instead.
            "shellextension" | "shell_extension" | "driver" => {
                return CleanLeftoverOutcome::skipped(
                    path_str.to_string(),
                    "manual removal only (system integration)",
                )
            }
            _ => {
                // Files, dirs, shortcuts, fonts, temp files, videos, dev
                // modules: decide by what actually exists on disk.
                let p = std::path::PathBuf::from(path_str);
                if !p.exists() {
                    return CleanLeftoverOutcome::deleted(
                        path_str.to_string(),
                        "already gone",
                    );
                }
                // Symlinks/junctions are never followed: refuse reparse points.
                if std::fs::symlink_metadata(&p)
                    .map(|m| m.file_type().is_symlink())
                    .unwrap_or(false)
                {
                    return CleanLeftoverOutcome::skipped(
                        path_str.to_string(),
                        "reparse point — manual review",
                    );
                }
                if p.is_dir() {
                    crate::utils::delete_directory(&p)
                } else {
                    crate::utils::delete_file(&p)
                }
            }
        };
        match res {
            Ok(()) => CleanLeftoverOutcome::deleted(path_str.to_string(), "deleted"),
            Err(e) => CleanLeftoverOutcome::skipped(path_str.to_string(), e.to_string()),
        }
    }

    /// Stop and delete an orphaned Windows service by name.
    async fn delete_orphan_service(&self, name: &str) -> Result<()> {
        #[cfg(all(target_os = "windows", feature = "windows"))]
        {
            let mgr = greek_windows::services::WindowsServiceManager::new();
            // Best-effort stop; delete is the goal.
            let _ = mgr.stop_service(name).await;
            mgr.delete_service(name).await?;
            Ok(())
        }
        #[cfg(not(all(target_os = "windows", feature = "windows")))]
        {
            let _ = name;
            Err(greek_common::GreekError::SystemError(
                "Service deletion is only supported on Windows".to_string(),
            ))
        }
    }

    /// Create a batch queue
    pub fn create_batch(&self, options: UninstallOptions) -> BatchQueue {
        BatchQueue::new(options)
    }

    /// Undo a previously recorded uninstall transaction, restoring backed-up
    /// files and registry keys.
    pub async fn undo_uninstall(&self, backup_id: uuid::Uuid) -> Result<()> {
        let transactions = crate::backup::list_transactions()?;
        let tx = transactions
            .into_iter()
            .find(|t| t.id == backup_id)
            .ok_or_else(|| {
                GreekError::NotFound(format!("No backup transaction found for {}", backup_id))
            })?;
        tx.rollback()
    }

    /// List recorded uninstall transactions that can still be undone.
    pub fn list_undoable_transactions(&self) -> Result<Vec<crate::backup::UninstallTransaction>> {
        crate::backup::list_transactions()
    }

    fn state_file_path() -> std::path::PathBuf {
        std::env::temp_dir().join(".reek_state.json")
    }

    fn persist_batch_state(batch: &BatchQueue) -> Result<()> {
        let p = Self::state_file_path();
        let json = serde_json::to_string_pretty(batch)
            .map_err(|e| GreekError::IoError(std::io::Error::other(e)))?;
        // Secure temp file: write then restrict permissions (600 on Unix)
        std::fs::write(&p, json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    fn clear_batch_state() {
        let _ = std::fs::remove_file(Self::state_file_path());
    }

    /// Execute a batch queue
    pub async fn execute_batch(&self, batch: &mut BatchQueue) -> Result<Vec<UninstallResult>> {
        let total = batch.items.len();
        tracing::info!("Starting batch uninstall of {} applications", total);
        // Persist state for resume/rollback (audit §6.2)
        let _ = Self::persist_batch_state(batch);

        let mut results = Vec::new();

        for (index, item) in batch.items.iter_mut().enumerate() {
            item.status = greek_common::BatchStatus::InProgress;

            let _ = self.event_sender.send(AppEvent::BatchProgress {
                completed: index,
                total,
                current_app: item.app.name.clone(),
            });

            match self.uninstall_app(&item.app, batch.options.clone()).await {
                Ok(result) => {
                    item.status = greek_common::BatchStatus::Completed;
                    item.result = Some(result.clone());
                    results.push(result);
                }
                Err(e) => {
                    item.status = greek_common::BatchStatus::Failed;
                    tracing::error!("Batch uninstall failed for {}: {}", item.app.name, e);

                    let failed_result = UninstallResult {
                        app_id: item.app.id,
                        success: false,
                        strategy_used: "batch".to_string(),
                        errors: vec![e.to_string()],
                        ..Default::default()
                    };

                    item.result = Some(failed_result.clone());
                    results.push(failed_result);
                }
            }
        }

        let _ = self.event_sender.send(AppEvent::BatchProgress {
            completed: batch.items.len(),
            total: batch.items.len(),
            current_app: "Complete".to_string(),
        });

        tracing::info!("Batch uninstall completed");
        Self::clear_batch_state();

        Ok(results)
    }

    /// Subscribe to real-time events
    pub fn subscribe_events(&self) -> broadcast::Receiver<AppEvent> {
        self.event_sender.subscribe()
    }

    /// Get current configuration
    pub fn config(&self) -> &GreekConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: GreekConfig) -> Result<()> {
        self.config = config;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use greek_common::{InstallSource, RegistryHive};

    fn fake_app(name: &str) -> InstalledApp {
        InstalledApp::new(
            name.to_string(),
            InstallSource::Registry {
                hive: RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        )
    }

    /// Writable scratch dir outside every PROTECTED_PATHS prefix.
    /// `TempDir` lives under `C:\Users` (protected by design), so positive
    /// deletion tests need a drive-root scratch dir. Skips gracefully where
    /// the root is not writable.
    fn writable_scratch(tag: &str) -> Option<std::path::PathBuf> {
        let dir = std::path::PathBuf::from(format!(
            "C:\\reek-test-{}-{}",
            std::process::id(),
            tag
        ));
        match std::fs::create_dir_all(&dir) {
            Ok(()) => Some(dir),
            Err(e) => {
                eprintln!("SKIP: drive-root scratch not writable ({e})");
                None
            }
        }
    }

    #[tokio::test]
    async fn test_force_remove_skips_vendor_uninstaller() {
        let Some(scratch) = writable_scratch("force") else { return };
        let target = scratch.join("FakeApp");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app.exe"), b"x").unwrap();
        let mut app = fake_app("FakeAppForForceTest");
        // A vendor uninstaller that can never succeed: nonexistent binary.
        // True force must never execute it.
        app.uninstall_string =
            Some("C:\\nonexistent\\uninstall-xyz123.exe /S".to_string());
        app.install_location = Some(target.clone());
        let svc = GreekAppService::new(GreekConfig::default()).unwrap();
        let res = svc
            .force_remove_app(&app, UninstallOptions::force())
            .await
            .unwrap();
        assert!(res.success);
        assert_eq!(res.strategy_used, "force-remove");
        assert!(!target.exists());
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[tokio::test]
    async fn test_clean_leftover_paths_deletes_file_and_blocks_protected() {
        let svc = GreekAppService::new(GreekConfig::default()).unwrap();
        // Pure-string cases: no filesystem needed.
        let out = svc
            .clean_leftover_paths(
                vec![
                    CleanLeftoverRequest {
                        path: "C:\\reek-definitely-missing-xyz\\gone.dll".to_string(),
                        artifact_type: "File".to_string(),
                        safety: "Caution".to_string(),
                    },
                    CleanLeftoverRequest {
                        path: "C:\\Windows\\System32\\evil.dll".to_string(),
                        artifact_type: "File".to_string(),
                        safety: "Safe".to_string(),
                    },
                ],
                false,
            )
            .await;
        assert!(out[0].deleted, "missing file must report already-gone");
        assert!(!out[1].deleted, "protected path must be refused");
        // Real deletion where the platform allows it.
        if let Some(scratch) = writable_scratch("clean") {
            let f = scratch.join("leftover.dll");
            std::fs::write(&f, b"x").unwrap();
            let out = svc
                .clean_leftover_paths(
                    vec![CleanLeftoverRequest {
                        path: f.to_string_lossy().to_string(),
                        artifact_type: "File".to_string(),
                        safety: "Safe".to_string(),
                    }],
                    false,
                )
                .await;
            assert!(out[0].deleted, "expected deletion, got {:?}", out[0]);
            assert!(!f.exists());
            let _ = std::fs::remove_dir_all(&scratch);
        }
    }

    #[tokio::test]
    async fn test_clean_leftover_paths_only_safe_gate() {
        let svc = GreekAppService::new(GreekConfig::default()).unwrap();
        // Gate is evaluated before any filesystem touch: use a missing path.
        let req = || CleanLeftoverRequest {
            path: "C:\\reek-definitely-missing-xyz\\risky.dat".to_string(),
            artifact_type: "File".to_string(),
            safety: "Caution".to_string(),
        };
        let gated = svc.clean_leftover_paths(vec![req()], true).await;
        assert!(!gated[0].deleted, "only_safe must skip Caution items");
        assert!(
            gated[0].reason.contains("confirmation"),
            "unexpected reason: {}",
            gated[0].reason
        );
        let explicit = svc.clean_leftover_paths(vec![req()], false).await;
        assert!(explicit[0].deleted, "explicit clean must delete");
    }
}
