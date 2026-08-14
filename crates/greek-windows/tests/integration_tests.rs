// Integration tests for greek-windows

#[cfg(target_os = "windows")]
use greek_common::AppScanner;
#[cfg(target_os = "windows")]
use greek_windows::*;

#[cfg(target_os = "windows")]
#[test]
fn test_windows_registry_scanner() {
    let scanner = WindowsRegistryScanner::new();
    let scanner_with = WindowsRegistryScanner::with_options(true, true, false);

    // Both constructors should produce usable scanners
    assert!(scanner.scanner_id() == "windows-registry");
    assert!(!scanner.scanner_name().is_empty());
    assert!(scanner_with.requires_elevation());
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_service_manager() {
    let manager = WindowsServiceManager::new();
    // Just test creation - actual operations require admin rights
    let _ = manager;
}

#[cfg(target_os = "windows")]
#[test]
fn test_restore_point_manager() {
    let manager = RestorePointManager::new();
    // Just test creation - actual operations require admin rights
    let _ = manager;
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_store_scanner() {
    let scanner = WindowsStoreScanner::new();
    let scanner_fw = WindowsStoreScanner::with_framework_apps(true);

    assert!(scanner.scanner_id() == "windows-store");
    assert!(!scanner.scanner_name().is_empty());
    let _ = scanner_fw;
}

#[cfg(target_os = "windows")]
#[test]
fn test_wmi_client() {
    let client = WmiClient::new();
    let _ = client;
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_windows_not_available() {
    // The actual Windows-specific code is behind cfg(target_os = "windows")
}
