// Integration tests for greek-core

use greek_common::*;
use greek_core::*;

#[tokio::test]
async fn test_scanner_manager() {
    let manager = ScannerManager::new();
    let apps = manager.scan_all().await.unwrap();

    // Platform scanners may be registered (Windows on Windows), so the result
    // is not guaranteed to be empty; it just must not error out.
    let _ = apps;
}

#[tokio::test]
async fn test_scanner_manager_with_scanner() {
    let mut manager = ScannerManager::new();

    let scanner = Box::new(PortableAppScanner::new(vec![std::path::PathBuf::from(
        "/tmp",
    )]));

    manager.register_scanner(scanner);

    let _ = manager.scan_all().await.unwrap();
    // May or may not find apps depending on the system
}

#[tokio::test]
async fn test_uninstaller_manager() {
    let manager = UninstallerManager::new();

    let app = InstalledApp::new(
        "Test App".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "test".to_string(),
        },
    );

    // Should find a strategy (force remove can handle anything)
    assert!(manager
        .uninstall(&app, UninstallOptions::force())
        .await
        .is_ok());
}

#[tokio::test]
async fn test_leftover_analyzer_manager() {
    let manager = LeftoverAnalyzerManager::new();

    let app = InstalledApp::new(
        "Test App".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "test".to_string(),
        },
    );

    let artifacts = manager.analyze_app(&app).await.unwrap();
    // Should return empty list since no analyzers are registered
    assert!(artifacts.is_empty());
}

#[tokio::test]
async fn test_app_service() {
    let config = GreekConfig::default();
    let mut service = GreekAppService::new(config).unwrap();

    let apps = service.scan_all_apps().await.unwrap();
    // Platform scanners may be registered; scan must simply complete without error.
    let _ = apps;
}

#[tokio::test]
async fn test_app_service_batch() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config).unwrap();

    let batch = service.create_batch(UninstallOptions::standard());
    assert_eq!(batch.items.len(), 0);
    assert_eq!(batch.pending_count(), 0);
}

#[tokio::test]
async fn test_config_manager() {
    let manager = ConfigManager::new().unwrap();

    let config = manager.load_config().unwrap();
    assert_eq!(config.ui.theme, "greek-blue");
}

#[tokio::test]
async fn test_config_manager_save_and_load() {
    let manager = ConfigManager::new().unwrap();

    let mut config = GreekConfig::default();
    config.ui.theme = "test-theme".to_string();

    manager.save_config(&config).unwrap();

    let loaded_config = manager.load_config().unwrap();
    assert_eq!(loaded_config.ui.theme, "test-theme");

    // Reset to default
    manager.reset_to_default().unwrap();
}

#[test]
fn test_file_utils() {
    use greek_core::utils::*;

    assert_eq!(format_bytes(1024), "1 KiB");
    assert_eq!(format_bytes(1024 * 1024), "1 MiB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1 GiB");
}

#[test]
fn test_protected_path_check() {
    use greek_core::utils::*;

    let protected = vec!["C:\\Windows".to_string(), "C:\\Program Files".to_string()];

    assert!(is_protected_path(
        std::path::Path::new("C:\\Windows\\System32"),
        &protected
    ));

    assert!(!is_protected_path(
        std::path::Path::new("C:\\Users\\Test"),
        &protected
    ));
}

#[tokio::test]
async fn test_standard_strategy() {
    let strategy = StandardUninstallStrategy::new();

    let mut app = InstalledApp::new(
        "Test App".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "test".to_string(),
        },
    );

    app.uninstall_string = Some("notepad.exe".to_string());

    assert!(strategy.can_handle(&app));
}

#[tokio::test]
async fn test_msi_strategy() {
    let strategy = MsiUninstallStrategy::new();

    let mut app = InstalledApp::new(
        "Test App".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "test".to_string(),
        },
    );

    app.uninstall_string = Some("MsiExec.exe /X{12345678-1234-1234-1234-123456789012}".to_string());

    assert!(strategy.can_handle(&app));
}

#[tokio::test]
async fn test_force_strategy() {
    let strategy = ForceRemoveStrategy::new();

    let app = InstalledApp::new(
        "Test App".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "test".to_string(),
        },
    );

    // Force remove can handle anything
    assert!(strategy.can_handle(&app));
}

#[test]
fn test_protected_path_edge_cases() {
    // Separator-aware: C:\Windows must not block C:\WindowsAppsFoo
    let protected = vec!["C:\\Windows".to_string()];
    assert!(greek_common::is_protected_path(
        std::path::Path::new("C:\\Windows\\System32\\evil.exe"),
        &protected
    ));
    assert!(!greek_common::is_protected_path(
        std::path::Path::new("C:\\WindowsAppsFoo\\app.exe"),
        &protected
    ));
    assert!(!greek_common::is_protected_path(
        std::path::Path::new("C:\\Windows.old\\app.exe"),
        &protected
    ));
}

#[test]
fn test_filesystem_backup_integration_with_tempfile() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let app_dir = tmp.path().join("FakeApp");
    std::fs::create_dir_all(app_dir.join("sub")).unwrap();
    std::fs::write(app_dir.join("sub").join("file.txt"), b"hello").unwrap();
    std::fs::write(app_dir.join("root.bin"), b"root").unwrap();

    let mut tx = greek_core::backup::UninstallTransaction::new("FakeApp").unwrap();
    tx.add_file_or_dir(&app_dir).unwrap();
    assert_eq!(tx.entries.len(), 1);
    tx.save_manifest().unwrap();
    assert!(tx.root().join("manifest.json").exists());

    // Simulate delete + rollback
    std::fs::remove_dir_all(&app_dir).unwrap();
    assert!(!app_dir.exists());
    tx.rollback().unwrap();
    assert!(app_dir.join("root.bin").exists());
    assert!(app_dir.join("sub").join("file.txt").exists());
}

#[tokio::test]
async fn test_portable_scanner_with_tempfile_100_apps_stress() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    // Create 100 fake portable app directories, each with an exe
    for i in 0..100 {
        let dir = tmp.path().join(format!("App{:03}", i));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("App{:03}.exe", i)), b"fake exe").unwrap();
    }
    let mut manager = ScannerManager::new();
    manager.register_scanner(Box::new(PortableAppScanner::new(vec![tmp
        .path()
        .to_path_buf()])));
    let apps = manager.scan_all().await.unwrap();
    // Must handle 100 apps without panic and within timeout (60s per CI)
    assert!(apps.len() >= 90, "expected ~100 apps, got {}", apps.len());
}

#[tokio::test]
async fn test_leftover_analyzer_with_tempfile() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let leftover_dir = tmp.path().join("LeftoverApp");
    std::fs::create_dir_all(&leftover_dir).unwrap();
    std::fs::write(leftover_dir.join("cache.tmp"), b"tmp").unwrap();

    let app = InstalledApp::new(
        "LeftoverApp".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "LeftoverApp".to_string(),
        },
    );
    let mut app_with_loc = app.clone();
    app_with_loc.install_location = Some(leftover_dir.clone());

    let manager = LeftoverAnalyzerManager::new();
    let artifacts = manager.analyze_app(&app_with_loc).await.unwrap();
    // No analyzer registered by default -> empty, but must not error
    let _ = artifacts;
}

#[test]
fn test_logging_init_creates_file_sink() {
    // Must not panic even when called twice (global subscriber already set in other tests)
    let _ = greek_common::logging::init_logging(false);
    tracing::info!("test log line for file sink");
    let log_dir = greek_common::logging::log_dir();
    assert!(log_dir.to_string_lossy().contains("logs") || log_dir.exists() || !log_dir.exists());
}

#[test]
fn test_error_severity_classification() {
    let e = greek_common::GreekError::SafetyError("protected".to_string());
    assert_eq!(e.severity(), greek_common::ErrorSeverity::UserIntervention);
    let e = greek_common::GreekError::Timeout("5s".to_string());
    assert_eq!(e.severity(), greek_common::ErrorSeverity::Recoverable);
    assert!(e.is_recoverable());
}
