// Configuration management for REEK Ultimate Uninstaller

use directories::ProjectDirs;
use greek_common::{GreekConfig, GreekError, Result};
use std::fs;
use std::path::PathBuf;

pub struct ConfigManager {
    config_dir: PathBuf,
    config_file: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "reek", "reek-uninstaller").ok_or_else(|| {
            GreekError::ConfigError("Failed to get project directories".to_string())
        })?;

        let config_dir = proj_dirs.config_dir().to_path_buf();
        let config_file = config_dir.join("config.toml");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        Ok(Self {
            config_dir,
            config_file,
        })
    }

    pub fn load_config(&self) -> Result<GreekConfig> {
        if !self.config_file.exists() {
            // Return default config if file doesn't exist
            return Ok(GreekConfig::default());
        }

        let content = fs::read_to_string(&self.config_file)?;
        let config: GreekConfig = toml::from_str(&content)?;

        // Validate config
        self.validate_config(&config)?;

        Ok(config)
    }

    pub fn save_config(&self, config: &GreekConfig) -> Result<()> {
        // Validate before saving
        self.validate_config(config)?;

        let content =
            toml::to_string_pretty(config).map_err(|e| GreekError::ConfigError(e.to_string()))?;
        fs::write(&self.config_file, content)
            .map_err(|e| GreekError::ConfigError(e.to_string()))?;

        Ok(())
    }

    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    pub fn config_file(&self) -> &PathBuf {
        &self.config_file
    }

    fn validate_config(&self, config: &GreekConfig) -> Result<()> {
        // Validate UI config
        if config.ui.animation_fps < 10 || config.ui.animation_fps > 60 {
            return Err(GreekError::ConfigError(
                "Animation FPS must be between 10 and 60".to_string(),
            ));
        }

        // Validate scanner config
        for dir in &config.scanner.scan_portable_dirs {
            if !PathBuf::from(dir).exists() {
                tracing::warn!("Portable scan directory does not exist: {}", dir);
            }
        }

        // Validate uninstall config
        if config.uninstall.default_timeout_seconds == 0 {
            return Err(GreekError::ConfigError(
                "Default timeout must be greater than 0".to_string(),
            ));
        }

        // Validate leftover config
        if config.leftover.confidence_threshold < 0.0 || config.leftover.confidence_threshold > 1.0
        {
            return Err(GreekError::ConfigError(
                "Confidence threshold must be between 0.0 and 1.0".to_string(),
            ));
        }

        // Validate backup config
        if config.backup.max_backup_size_mb == 0 {
            return Err(GreekError::ConfigError(
                "Max backup size must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    pub fn reset_to_default(&self) -> Result<()> {
        let default_config = GreekConfig::default();
        self.save_config(&default_config)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ConfigManager")
    }
}
