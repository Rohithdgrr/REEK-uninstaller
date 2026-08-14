// Integration tests for greek-core

use greek_common::*;
use greek_core::*;

#[tokio::test]
async fn test_scanner_manager() {
    let manager = ScannerManager::new();
    let apps = manager.scan_all().await.unwrap();
    
    // Should return empty list since no scanners are registered
    assert!(apps.is_empty());
}

#[tokio::test]
async fn test_scanner_manager_with_scanner() {
    let mut manager = ScannerManager::new();
    
    let scanner = Box::new(PortableAppScanner::new(vec![
        std::path::PathBuf::from("/tmp")
    ]));
    
    manager.register_scanner(scanner);
    
    let apps = manager.scan_all().await.unwrap();
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
            is_64_bit: true,
        },
    );
    
    // Should find a strategy (force remove can handle anything)
    assert!(manager.uninstall(&app, UninstallOptions::force()).await.is_ok());
}

#[tokio::test]
async fn test_leftover_analyzer_manager() {
    let manager = LeftoverAnalyzerManager::new();
    
    let app = InstalledApp::new(
        "Test App".to_string(),
        InstallSource::Registry {
            hive: RegistryHive::Hklm,
            key_path: "test".to_string(),
            is_64_bit: true,
        },
    );
    
    let artifacts = manager.analyze_app(&app).await.unwrap();
    // Should return empty list since no analyzers are registered
    assert!(artifacts.is_empty());
}

#[tokio::test]
async fn test_app_service() {
    let config = GreekConfig::default();
    let service = GreekAppService::new(config).unwrap();
    
    let apps = service.scan_all_apps().await.unwrap();
    // Empty since no scanners are registered
    assert!(apps.is_empty());
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
    
    let protected = vec![
        "C:\\Windows".to_string(),
        "C:\\Program Files".to_string(),
    ];
    
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
            is_64_bit: true,
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
            is_64_bit: true,
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
            is_64_bit: true,
        },
    );
    
    // Force remove can handle anything
    assert!(strategy.can_handle(&app));
}
