// Integration tests for greek-platform

use greek_platform::*;

#[test]
fn test_get_os() {
    let os = get_os();
    assert!(!os.is_empty());
    
    // Should match one of the known OS values
    let known_os = vec!["windows", "linux", "macos", "android", "ios"];
    assert!(known_os.contains(&os));
}

#[test]
fn test_get_arch() {
    let arch = get_arch();
    assert!(!arch.is_empty());
    
    // Should match one of the known architecture values
    let known_arch = vec!["x86", "x86_64", "arm", "aarch64", "mips", "powerpc", "powerpc64"];
    assert!(known_arch.contains(&arch));
}

#[test]
fn test_get_common_app_dirs() {
    let dirs = get_common_app_dirs();
    assert!(!dirs.is_empty());
}

#[test]
fn test_get_home_dir() {
    let home = get_home_dir();
    assert!(home.is_ok());
    
    if let Ok(home_path) = home {
        assert!(home_path.is_absolute());
    }
}

#[cfg(feature = "linux")]
#[tokio::test]
async fn test_linux_scanner() {
    use greek_common::*;
    
    let scanner = LinuxPackageScanner::new(LinuxPackageManager::Apt);
    
    let apps = scanner.scan().await.unwrap();
    // May or may not find apps depending on the system
}

#[cfg(not(feature = "linux"))]
#[tokio::test]
async fn test_linux_scanner_disabled() {
    use greek_common::*;
    
    let scanner = LinuxPackageScanner::new();
    
    let apps = scanner.scan().await.unwrap();
    assert!(apps.is_empty());
    
    assert!(!scanner.requires_elevation());
}
