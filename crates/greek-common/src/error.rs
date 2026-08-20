// Comprehensive error handling for REEK Ultimate Uninstaller

use thiserror::Error;

/// Core error type for the application
#[derive(Error, Debug)]
pub enum GreekError {
    #[error("Scanner error: {0}")]
    ScanError(String),

    #[error("Uninstall error: {0}")]
    UninstallError(String),

    #[error("Analysis error: {0}")]
    AnalysisError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("Registry error: {0}")]
    RegistryError(String),

    #[error("Permission denied: {0}")]
    PermissionError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Operation timeout: {0}")]
    Timeout(String),

    #[error("Process error: {0}")]
    ProcessError(String),

    #[error("Service error: {0}")]
    ServiceError(String),

    #[error("System error: {0}")]
    SystemError(String),

    #[error("Safety violation: {0}")]
    SafetyError(String),

    #[error("Backup error: {0}")]
    BackupError(String),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, GreekError>;

/// Scan-specific errors
#[derive(Error, Debug)]
pub enum ScanError {
    #[error("Registry scan failed: {0}")]
    RegistryScanFailed(String),

    #[error("Windows Store scan failed: {0}")]
    StoreScanFailed(String),

    #[error("File system scan failed: {0}")]
    FileSystemScanFailed(String),

    #[error("Permission denied during scan: {0}")]
    PermissionDenied(String),

    #[error("Invalid registry key: {0}")]
    InvalidRegistryKey(String),

    #[error("App data parsing failed: {0}")]
    ParseError(String),
}

/// Uninstall-specific errors
#[derive(Error, Debug)]
pub enum UninstallError {
    #[error("No uninstall strategy found for application")]
    NoStrategyFound,

    #[error("Uninstaller execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Process timeout after {0} seconds")]
    Timeout(u64),

    #[error("File in use: {0}")]
    FileInUse(String),

    #[error("Registry deletion failed: {0}")]
    RegistryDeletionFailed(String),

    #[error("Service operation failed: {0}")]
    ServiceOperationFailed(String),

    #[error("Restore point creation failed: {0}")]
    RestorePointFailed(String),

    #[error("Backup operation failed: {0}")]
    BackupFailed(String),

    #[error("Force remove failed: {0}")]
    ForceRemoveFailed(String),
}

/// Analysis-specific errors
#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("Leftover scan failed: {0}")]
    ScanFailed(String),

    #[error("Confidence scoring failed: {0}")]
    ScoringFailed(String),

    #[error("Path resolution failed: {0}")]
    PathResolutionFailed(String),

    #[error("Service analysis failed: {0}")]
    ServiceAnalysisFailed(String),

    #[error("Task analysis failed: {0}")]
    TaskAnalysisFailed(String),
}

/// Configuration-specific errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    FileNotFound(String),

    #[error("Invalid config format: {0}")]
    InvalidFormat(String),

    #[error("Invalid config value: {0}")]
    InvalidValue(String),

    #[error("Config validation failed: {0}")]
    ValidationFailed(String),
}

impl From<ScanError> for GreekError {
    fn from(err: ScanError) -> Self {
        GreekError::ScanError(err.to_string())
    }
}

impl From<UninstallError> for GreekError {
    fn from(err: UninstallError) -> Self {
        GreekError::UninstallError(err.to_string())
    }
}

impl From<AnalysisError> for GreekError {
    fn from(err: AnalysisError) -> Self {
        GreekError::AnalysisError(err.to_string())
    }
}

impl From<ConfigError> for GreekError {
    fn from(err: ConfigError) -> Self {
        GreekError::ConfigError(err.to_string())
    }
}
