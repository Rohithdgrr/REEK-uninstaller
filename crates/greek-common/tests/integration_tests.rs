// Integration tests for greek-common

use greek_common::*;

#[test]
fn test_installed_app_serialization() {
    let app = InstalledApp::new(
        "Test App".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "test".to_string(),
        },
    );

    let serialized = serde_json::to_string(&app).unwrap();
    let deserialized: InstalledApp = serde_json::from_str(&serialized).unwrap();

    assert_eq!(app.name, deserialized.name);
    assert_eq!(app.id, deserialized.id);
}

#[test]
fn test_greek_config_serialization() {
    let config = GreekConfig::default();

    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: GreekConfig = serde_json::from_str(&serialized).unwrap();

    assert_eq!(config.ui.theme, deserialized.ui.theme);
    assert_eq!(
        config.scanner.scan_browser_extensions,
        deserialized.scanner.scan_browser_extensions
    );
}

#[test]
fn test_uninstall_options_presets() {
    let standard = UninstallOptions::standard();
    assert!(!standard.silent);
    assert!(!standard.force);
    assert!(standard.create_restore_point);

    let silent = UninstallOptions::silent();
    assert!(silent.silent);
    assert!(!silent.force);

    let force = UninstallOptions::force();
    assert!(force.force);
    assert!(force.delete_leftovers);
}

#[test]
fn test_batch_queue_operations() {
    let options = UninstallOptions::standard();
    let mut queue = BatchQueue::new(options);

    let app1 = InstalledApp::new(
        "App 1".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "test1".to_string(),
        },
    );

    let app2 = InstalledApp::new(
        "App 2".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "test2".to_string(),
        },
    );

    queue.add_item(app1);
    queue.add_item(app2);

    assert_eq!(queue.pending_count(), 2);
    assert_eq!(queue.completed_count(), 0);
    assert_eq!(queue.failed_count(), 0);
}

#[test]
fn test_safety_level_ordering() {
    assert!(SafetyLevel::Safe < SafetyLevel::Caution);
    assert!(SafetyLevel::Caution < SafetyLevel::Dangerous);
    assert!(SafetyLevel::Dangerous < SafetyLevel::Critical);
}

#[test]
fn test_leftover_artifact_safety_check() {
    let mut artifact =
        LeftoverArtifact::new(ArtifactType::Directory, std::path::PathBuf::from("/test"));

    artifact.safety_level = SafetyLevel::Safe;
    assert!(artifact.is_safe_to_delete());

    artifact.safety_level = SafetyLevel::Critical;
    assert!(!artifact.is_safe_to_delete());
}

#[test]
fn test_registry_key_display() {
    assert_eq!(RegistryHive::Hklm.as_str(), "HKLM");
    assert_eq!(RegistryHive::Hkcu.as_str(), "HKCU");
}

#[test]
fn test_error_conversions() {
    let scan_error = ScanError::RegistryScanFailed("test".to_string());
    let greek_error: GreekError = scan_error.into();
    assert!(matches!(greek_error, GreekError::ScanError(_)));

    let uninstall_error = UninstallError::ExecutionFailed("test".to_string());
    let greek_error: GreekError = uninstall_error.into();
    assert!(matches!(greek_error, GreekError::UninstallError(_)));

    let analysis_error = AnalysisError::ScanFailed("test".to_string());
    let greek_error: GreekError = analysis_error.into();
    assert!(matches!(greek_error, GreekError::AnalysisError(_)));

    let config_error = ConfigError::FileNotFound("test".to_string());
    let greek_error: GreekError = config_error.into();
    assert!(matches!(greek_error, GreekError::ConfigError(_)));
}

#[test]
fn test_theme_parsing() {
    let theme = Theme::default();
    assert_eq!(theme.name, "greek-blue");
    assert!(!theme.background.is_empty());
    assert!(!theme.foreground.is_empty());
}

#[test]
fn test_config_defaults() {
    let config = GreekConfig::default();

    assert_eq!(config.ui.theme, "greek-blue");
    assert!(config.ui.show_icons);
    assert!(config.ui.confirm_destructive);

    assert_eq!(config.uninstall.default_timeout_seconds, 300);
    assert!(config.uninstall.kill_processes_before_uninstall);

    assert_eq!(config.leftover.aggressiveness, Aggressiveness::Normal);
    assert_eq!(config.leftover.confidence_threshold, 0.7);

    assert!(config.backup.backup_registry);
    assert_eq!(config.backup.max_backup_size_mb, 100);
}
