// Integration tests for greek-platform

use greek_platform::*;

#[test]
fn test_get_os() {
    let os = get_os();
    assert!(!os.is_empty());

    // Should match one of the known OS values
    let known_os = ["windows", "linux", "macos", "android", "ios"];
    assert!(known_os.contains(&os));
}

#[test]
fn test_get_arch() {
    let arch = get_arch();
    assert!(!arch.is_empty());

    // Should match one of the known architecture values
    let known_arch = [
        "x86",
        "x86_64",
        "arm",
        "aarch64",
        "mips",
        "powerpc",
        "powerpc64",
    ];
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

#[tokio::test]
async fn test_linux_scanner() {
    let scanner = LinuxPackageScanner::new(LinuxPackageManager::Apt);

    // The command-based scan may fail if apt isn't installed, so we only
    // verify the scanner can be constructed and invoked.
    let _ = scanner.scan().await;
}
