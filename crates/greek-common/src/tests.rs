// Comprehensive unit tests for greek-common

use crate::error::*;
use crate::models::*;

#[cfg(test)]
mod model_tests {
    use super::*;

    #[test]
    fn test_installed_app_creation() {
        let app = InstalledApp::new(
            "Test App".to_string(),
            InstallSource::Registry {
                hive: RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        );

        assert_eq!(app.name, "Test App");
        assert!(app.publisher.is_none());
        assert!(app.version.is_none());
    }

    #[test]
    fn test_installed_app_display_name() {
        let mut app = InstalledApp::new(
            "Test App".to_string(),
            InstallSource::Registry {
                hive: RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        );

        app.version = Some("1.0.0".to_string());
        assert_eq!(app.display_name(), "Test App 1.0.0");
    }

    #[test]
    fn test_installed_app_display_size() {
        let mut app = InstalledApp::new(
            "Test App".to_string(),
            InstallSource::Registry {
                hive: RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        );

        app.size_bytes = Some(1024 * 1024);
        assert_eq!(app.display_size(), Some("1 MiB".to_string()));
    }

    #[test]
    fn test_has_removal_path() {
        // Bare entry: nothing to act on -> hidden from the desktop list.
        let bare = InstalledApp::new(
            "Ghost Entry".to_string(),
            InstallSource::Registry {
                hive: RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        );
        assert!(!bare.has_removal_path());

        // Vendor uninstall string suffices.
        let mut s = bare.clone();
        s.uninstall_string = Some("C:\\App\\uninstall.exe /S".to_string());
        assert!(s.has_removal_path());

        // Quiet / modify strings also count as a vendor uninstaller.
        let mut q = bare.clone();
        q.quiet_uninstall_string = Some("C:\\App\\uninstall.exe /S".to_string());
        assert!(q.has_removal_path());
        let mut m = bare.clone();
        m.modify_string = Some("C:\\App\\setup.exe /modify".to_string());
        assert!(m.has_removal_path());

        // Install location alone is NOT enough for Registry apps (no real
        // uninstaller — those entries fail at click time, so they stay hidden).
        let mut l = bare.clone();
        l.install_location = Some(std::path::PathBuf::from("C:\\Apps\\Thing"));
        assert!(!l.has_removal_path());

        // Registry keys alone are NOT enough either.
        let mut r = bare.clone();
        r.registry_keys = vec![crate::models::RegistryKey {
            hive: RegistryHive::Hklm,
            path: "HKLM\\Software\\Thing".to_string(),
            values: std::collections::HashMap::new(),
        }];
        assert!(!r.has_removal_path());

        // Portable apps ARE deletable via their folder.
        let mut p = InstalledApp::new(
            "Portable Thing".to_string(),
            InstallSource::Portable {
                detected_path: std::path::PathBuf::from("C:\\Apps\\Thing"),
                confidence: 0.9,
            },
        );
        p.install_location = Some(std::path::PathBuf::from("C:\\Apps\\Thing"));
        assert!(p.has_removal_path());

        // Store package with family name is removable via Remove-AppxPackage.
        let store = InstalledApp::new(
            "Store Thing".to_string(),
            InstallSource::WindowsStore {
                package_family_name: "Thing_abc123".to_string(),
                package_full_name: "Thing_1.0_abc123".to_string(),
            },
        );
        assert!(store.has_removal_path());

        // Store entry without identity is not.
        let store_empty = InstalledApp::new(
            "Store Ghost".to_string(),
            InstallSource::WindowsStore {
                package_family_name: String::new(),
                package_full_name: String::new(),
            },
        );
        assert!(!store_empty.has_removal_path());
    }

    #[test]
    fn test_leftover_artifact_creation() {
        let artifact =
            LeftoverArtifact::new(ArtifactType::Directory, std::path::PathBuf::from("/test"));

        assert_eq!(artifact.artifact_type, ArtifactType::Directory);
        assert_eq!(artifact.confidence, 0.5);
        assert_eq!(artifact.safety_level, SafetyLevel::Caution);
    }

    #[test]
    fn test_leftover_artifact_safety() {
        let mut artifact =
            LeftoverArtifact::new(ArtifactType::Directory, std::path::PathBuf::from("/test"));
        artifact.safety_level = SafetyLevel::Safe;

        assert!(artifact.is_safe_to_delete());
    }

    #[test]
    fn test_uninstall_options_standard() {
        let options = UninstallOptions::standard();

        assert!(!options.silent);
        assert!(!options.force);
        assert!(options.create_restore_point);
        assert!(options.backup_registry);
    }

    #[test]
    fn test_uninstall_options_silent() {
        let options = UninstallOptions::silent();

        assert!(options.silent);
        assert!(!options.force);
    }

    #[test]
    fn test_uninstall_options_force() {
        let options = UninstallOptions::force();

        assert!(options.force);
        assert!(options.delete_leftovers);
    }

    #[test]
    fn test_batch_queue() {
        let options = UninstallOptions::standard();
        let mut queue = BatchQueue::new(options);

        let app = InstalledApp::new(
            "Test App".to_string(),
            InstallSource::Registry {
                hive: RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        );

        queue.add_item(app);

        assert_eq!(queue.items.len(), 1);
        assert_eq!(queue.pending_count(), 1);
    }

    #[test]
    fn test_greek_config_default() {
        let config = GreekConfig::default();

        assert_eq!(config.ui.theme, "greek-blue");
        assert!(config.ui.show_icons);
        assert!(config.ui.confirm_destructive);
    }

    #[test]
    fn test_safety_level_ord() {
        assert!(SafetyLevel::Safe < SafetyLevel::Caution);
        assert!(SafetyLevel::Caution < SafetyLevel::Dangerous);
        assert!(SafetyLevel::Dangerous < SafetyLevel::Critical);
    }

    #[test]
    fn test_registry_hive_as_str() {
        assert_eq!(RegistryHive::Hklm.as_str(), "HKLM");
        assert_eq!(RegistryHive::Hkcu.as_str(), "HKCU");
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_greek_error_display() {
        let error = GreekError::ScanError("Test error".to_string());
        assert_eq!(error.to_string(), "Scanner error: Test error");
    }

    #[test]
    fn test_scan_error_display() {
        let error = ScanError::RegistryScanFailed("Test error".to_string());
        assert_eq!(error.to_string(), "Registry scan failed: Test error");
    }

    #[test]
    fn test_uninstall_error_display() {
        let error = UninstallError::ExecutionFailed("Test error".to_string());
        assert_eq!(
            error.to_string(),
            "Uninstaller execution failed: Test error"
        );
    }

    #[test]
    fn test_error_conversions() {
        let scan_error = ScanError::FileSystemScanFailed("Test".to_string());
        let greek_error: GreekError = scan_error.into();

        assert!(matches!(greek_error, GreekError::ScanError(_)));
    }
}
