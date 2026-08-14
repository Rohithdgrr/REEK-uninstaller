// Integration tests for greek-windows

#[cfg(target_os = "windows")]
use greek_windows::*;

#[cfg(target_os = "windows")]
#[test]
fn test_windows_registry_scanner() {
    let scanner = WindowsRegistryScanner::new();
    
    assert!(scanner.scan_64bit);
    assert!(scanner.scan_32bit);
    assert!(!scanner.include_system_components);
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_registry_scanner_options() {
    let scanner = WindowsRegistryScanner::with_options(true, false, true);
    
    assert!(scanner.scan_64bit);
    assert!(!scanner.scan_32bit);
    assert!(scanner.include_system_components);
}

#[cfg(target_os = "windows")]
#[test]
fn test_date_parsing() {
    let scanner = WindowsRegistryScanner::new();
    
    // Valid date
    let date = scanner.parse_install_date("20240315");
    assert!(date.is_some());
    assert_eq!(date.unwrap(), chrono::NaiveDate::from_ymd_opt(2024, 3, 15).unwrap());
    
    // Invalid date
    let date = scanner.parse_install_date("invalid");
    assert!(date.is_none());
    
    // Short date
    let date = scanner.parse_install_date("2024");
    assert!(date.is_none());
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_service_manager() {
    let manager = WindowsServiceManager::new();
    // Just test creation - actual operations require admin rights
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_restore_manager() {
    let manager = WindowsRestoreManager::new();
    // Just test creation - actual operations require admin rights
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_store_scanner() {
    let scanner = WindowsStoreScanner::new();
    
    assert!(!scanner.include_provisioned);
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_store_scanner_options() {
    let scanner = WindowsStoreScanner::with_options(true);
    
    assert!(scanner.include_provisioned);
}

#[cfg(target_os = "windows")]
#[test]
fn test_wmi_helper() {
    let helper = WmiHelper::new();
    assert!(helper.is_ok());
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_windows_not_available() {
    // These tests should compile but not run on non-Windows platforms
    // The actual Windows-specific code is behind cfg(target_os = "windows")
}
