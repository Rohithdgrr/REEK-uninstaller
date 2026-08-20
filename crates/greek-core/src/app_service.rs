// Core application service coordinating all uninstallation operations

use crate::config::ConfigManager;
use crate::leftover::LeftoverAnalyzerManager;
use crate::scanner::ScannerManager;
use crate::uninstaller::UninstallerManager;
use greek_common::{
    AppEvent, ArtifactType, BatchQueue, GreekConfig, GreekError, InstalledApp, LeftoverArtifact,
    Result, SafetyLevel, UninstallOptions, UninstallResult,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

#[cfg(all(target_os = "windows", feature = "windows"))]
use greek_windows::icon::IconExtractor;

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

        // Register filesystem leftover analyzer
        {
            use crate::leftover::FileSystemLeftoverAnalyzer;
            use std::path::PathBuf;

            let common_dirs = vec![
                PathBuf::from("C:\\ProgramData"),
                PathBuf::from(
                    std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users".to_string()),
                ),
            ];

            leftover_analyzer_manager
                .register_analyzer(Box::new(FileSystemLeftoverAnalyzer::new(common_dirs)));
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
        let apps = tokio::task::spawn_blocking(move || {
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

        let _ = self.event_sender.send(AppEvent::ScanCompleted {
            scanner_id: "all".to_string(),
            count: apps.len(),
        });

        tracing::info!("System scan completed, found {} applications", apps.len());

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

    /// Force remove an application
    pub async fn force_remove_app(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult> {
        tracing::warn!("Starting force remove for: {}", app.name);

        let mut force_options = options;
        force_options.force = true;

        self.uninstall_app(app, force_options).await
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
            let artifact = self
                .artifact_cache
                .get(artifact_id)
                .ok_or_else(|| {
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
                        tracing::info!(
                            "Deleted leftover directory: {}",
                            artifact.path.display()
                        );
                    }
                }
                ArtifactType::RegistryKey => {
                    crate::utils::delete_registry_key(
                        &artifact.path.to_string_lossy(),
                    )?;
                    tracing::info!(
                        "Deleted leftover registry key: {}",
                        artifact.path.display()
                    );
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

    /// Execute a batch queue
    pub async fn execute_batch(&self, batch: &mut BatchQueue) -> Result<Vec<UninstallResult>> {
        let total = batch.items.len();
        tracing::info!("Starting batch uninstall of {} applications", total);

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
