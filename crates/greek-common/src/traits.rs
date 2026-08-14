// Core traits for the REEK Ultimate Uninstaller system

use crate::error::Result;
use crate::models::*;
use async_trait::async_trait;

/// Trait for any component that can discover installed applications
#[async_trait]
pub trait AppScanner: Send + Sync {
    /// Unique identifier for this scanner
    fn scanner_id(&self) -> &'static str;

    /// Human-readable name
    fn scanner_name(&self) -> String;

    /// Scan for installed applications
    async fn scan(&self) -> Result<Vec<InstalledApp>>;

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
    ) -> Result<UninstallResult>;

    /// Attempt silent uninstallation
    async fn uninstall_silent(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult>;
}

/// Trait for leftover artifact detection
#[async_trait]
pub trait LeftoverAnalyzer: Send + Sync {
    /// Analyzer identifier
    fn analyzer_id(&self) -> &'static str;

    /// Analyze an app for leftover artifacts
    async fn analyze(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>>;

    /// Analyze the entire system for orphaned artifacts
    async fn analyze_system(&self) -> Result<Vec<LeftoverArtifact>>;

    /// Score the confidence that an artifact belongs to an app
    fn score_confidence(&self, artifact: &LeftoverArtifact, app: &InstalledApp) -> f32;
}

/// Trait for backup and restore operations
#[async_trait]
pub trait BackupManager: Send + Sync {
    /// Create a backup of the specified resource
    async fn create_backup(&self, resource: &str) -> Result<String>;

    /// Restore from a backup
    async fn restore_backup(&self, backup_id: &str) -> Result<()>;

    /// List available backups
    async fn list_backups(&self) -> Result<Vec<BackupInfo>>;

    /// Delete a backup
    async fn delete_backup(&self, backup_id: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub id: String,
    pub resource: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub size_bytes: u64,
}

/// Trait for system restore point operations
#[async_trait]
pub trait RestorePointManager: Send + Sync {
    /// Create a system restore point
    async fn create_restore_point(&self, description: &str) -> Result<String>;

    /// List available restore points
    async fn list_restore_points(&self) -> Result<Vec<RestorePointInfo>>;

    /// Restore to a specific restore point
    async fn restore(&self, restore_point_id: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct RestorePointInfo {
    pub id: String,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub sequence_number: u32,
}

/// Trait for process management
#[async_trait]
pub trait ProcessManager: Send + Sync {
    /// Find processes by executable name
    async fn find_processes_by_name(&self, name: &str) -> Result<Vec<u32>>;

    /// Find processes by path
    async fn find_processes_by_path(&self, path: &std::path::Path) -> Result<Vec<u32>>;

    /// Terminate a process
    async fn terminate_process(&self, pid: u32) -> Result<()>;

    /// Kill all processes for an app
    async fn kill_app_processes(&self, app: &InstalledApp) -> Result<Vec<u32>>;
}

/// Trait for service management
#[async_trait]
pub trait ServiceManager: Send + Sync {
    /// Find services by app path
    async fn find_services_by_app(&self, app: &InstalledApp) -> Result<Vec<String>>;

    /// Stop a service
    async fn stop_service(&self, service_name: &str) -> Result<()>;

    /// Delete a service
    async fn delete_service(&self, service_name: &str) -> Result<()>;

    /// Stop and delete all services for an app
    async fn cleanup_app_services(&self, app: &InstalledApp) -> Result<Vec<String>>;
}

/// Trait for scheduled task management
#[async_trait]
pub trait TaskManager: Send + Sync {
    /// Find tasks by app
    async fn find_tasks_by_app(&self, app: &InstalledApp) -> Result<Vec<String>>;

    /// Delete a task
    async fn delete_task(&self, task_name: &str) -> Result<()>;

    /// Delete all tasks for an app
    async fn cleanup_app_tasks(&self, app: &InstalledApp) -> Result<Vec<String>>;
}

/// Trait for system statistics collection (cross-platform)
#[async_trait]
pub trait SystemStatsProvider: Send + Sync {
    /// Collect current system stats
    fn collect(&mut self) -> crate::models::SystemStats;

    /// Check if running with elevated privileges
    fn is_elevated(&self) -> bool;
}
