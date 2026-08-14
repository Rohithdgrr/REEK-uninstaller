// Core application service coordinating all uninstallation operations

use greek_common::{
    GreekConfig, GreekError, Result, InstalledApp, UninstallOptions, UninstallResult,
    LeftoverArtifact, BatchQueue, AppEvent,
};
use tokio::sync::broadcast;
use crate::scanner::ScannerManager;
use crate::uninstaller::UninstallerManager;
use crate::leftover::LeftoverAnalyzerManager;
use crate::config::ConfigManager;

/// Main service that coordinates all uninstaller operations
pub struct GreekAppService {
    config: GreekConfig,
    scanner_manager: ScannerManager,
    uninstaller_manager: UninstallerManager,
    leftover_analyzer_manager: LeftoverAnalyzerManager,
    event_sender: broadcast::Sender<AppEvent>,
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
            
            let scan_dirs: Vec<PathBuf> = config.scanner.scan_portable_dirs
                .iter()
                .map(|s| PathBuf::from(s))
                .collect();
            
            scanner_manager.register_scanner(Box::new(PortableAppScanner::new(scan_dirs)));
        }
        
        // Register filesystem leftover analyzer
        {
            use crate::leftover::FileSystemLeftoverAnalyzer;
            use std::path::PathBuf;
            
            let common_dirs = vec![
                PathBuf::from("C:\\ProgramData"),
                PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users".to_string())),
            ];
            
            leftover_analyzer_manager.register_analyzer(Box::new(FileSystemLeftoverAnalyzer::new(common_dirs)));
        }
        
        tracing::info!(
            "GreekAppService initialized with {} scanners",
            scanner_manager.scanner_count()
        );
        for name in scanner_manager.scanner_names() {
            tracing::info!("  - Scanner: {}", name);
        }
        
        let service = Self {
            config,
            scanner_manager,
            uninstaller_manager,
            leftover_analyzer_manager,
            event_sender,
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

    /// Scan all sources for installed applications
    pub async fn scan_all_apps(&self) -> Result<Vec<InstalledApp>> {
        tracing::info!("Starting full system scan");
        
        let _ = self.event_sender.send(AppEvent::ScanStarted {
            scanner_id: "all".to_string(),
        });
        
        let mut apps = self.scanner_manager.scan_all().await
            .map_err(|e| GreekError::ScanError(e.to_string()))?;
        
        // Enrich apps: extract real icons, compute dominant colors, fill missing sizes
        let apps = tokio::task::spawn_blocking(move || {
            #[cfg(windows)]
            {
                let extractor = greek_windows::icon::IconExtractor::new();
                extractor.enrich_apps(&mut apps);
            }
            #[cfg(not(windows))]
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
        
        Ok(apps)
    }

    /// Get detailed info about a specific app
    pub async fn get_app_details(&self, app_id: uuid::Uuid) -> Result<InstalledApp> {
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
        
        let mut result = self.uninstaller_manager.uninstall(app, options).await
            .map_err(|e| GreekError::UninstallError(e.to_string()))?;
        
        let _ = self.event_sender.send(AppEvent::UninstallCompleted {
            app_id: app.id,
            result: result.clone(),
        });
        
        tracing::info!("Uninstall completed for: {} - Success: {}", app.name, result.success);
        
        Ok(result)
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

    /// Analyze leftovers for an app
    pub async fn analyze_leftovers(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        tracing::info!("Analyzing leftovers for: {}", app.name);
        
        let _ = self.event_sender.send(AppEvent::LeftoverScanStarted {
            app_id: app.id,
        });
        
        let artifacts = self.leftover_analyzer_manager.analyze_app(app).await
            .map_err(|e| GreekError::AnalysisError(e.to_string()))?;
        
        for artifact in &artifacts {
            let _ = self.event_sender.send(AppEvent::LeftoverFound {
                app_id: app.id,
                artifact: artifact.clone(),
            });
        }
        
        tracing::info!("Leftover analysis completed for: {}, found {} artifacts", app.name, artifacts.len());
        
        Ok(artifacts)
    }

    /// Clean up leftovers
    pub async fn clean_leftovers(
        &self,
        artifact_ids: Vec<uuid::Uuid>,
        _options: UninstallOptions,
    ) -> Result<()> {
        tracing::info!("Cleaning up {} leftover artifacts", artifact_ids.len());
        
        for artifact_id in artifact_ids {
            tracing::info!("Would delete artifact: {}", artifact_id);
        }
        
        Ok(())
    }

    /// Create a batch queue
    pub fn create_batch(&self, options: UninstallOptions) -> BatchQueue {
        BatchQueue::new(options)
    }

    /// Execute a batch queue
    pub async fn execute_batch(
        &self,
        batch: &mut BatchQueue,
    ) -> Result<Vec<UninstallResult>> {
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
