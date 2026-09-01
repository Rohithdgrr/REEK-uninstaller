// Utility functions for REEK Ultimate Uninstaller

use greek_common::{GreekError, Result};
use humansize::{format_size, BINARY};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn calculate_file_hash(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

pub fn format_bytes(bytes: u64) -> String {
    format_size(bytes, BINARY)
}

pub fn normalize_path(path: &Path) -> PathBuf {
    // Convert to absolute path and normalize
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

/// Delegate to the canonical implementation in `greek_common`.
pub fn is_protected_path(path: &Path, protected_paths: &[String]) -> bool {
    greek_common::is_protected_path(path, protected_paths)
}

/// Check whether a registry key path is protected and must not be deleted.
///
/// Uses case-insensitive, separator-aware prefix matching:
/// `HKLM\SYSTEM` blocks `HKLM\SYSTEM\CurrentControlSet` but NOT
/// `HKLM\SOFTWARE\Microsoft\WindowsAppsFoo` (suffix without separator).
#[cfg(target_os = "windows")]
pub fn is_protected_registry_path(reg_path: &str) -> bool {
    fn norm(s: &str) -> String {
        let mut n = s.to_lowercase();
        while n.len() > 1 && n.ends_with('\\') {
            n.pop();
        }
        n
    }
    let lower = norm(reg_path);
    for protected in greek_common::PROTECTED_REGISTRY_PATHS {
        let prot = norm(protected);
        if lower == prot {
            return true;
        }
        if lower.starts_with(&(prot.clone() + "\\")) {
            return true;
        }
    }
    false
}

#[cfg(not(target_os = "windows"))]
pub fn is_protected_registry_path(_reg_path: &str) -> bool {
    false
}

pub fn ensure_directory_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn get_file_size(path: &Path) -> Result<u64> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

pub fn get_directory_size(path: &Path) -> Result<u64> {
    let mut total_size = 0u64;

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }
    }

    Ok(total_size)
}

pub fn delete_file(path: &Path) -> Result<()> {
    std::fs::remove_file(path)?;
    Ok(())
}

pub fn delete_directory(path: &Path) -> Result<()> {
    // Safety: refuse to delete protected system paths.
    let protected = greek_common::PROTECTED_PATHS
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    if is_protected_path(path, &protected) {
        return Err(GreekError::SafetyError(format!(
            "Refusing to delete protected system path: {}",
            path.display()
        )));
    }
    std::fs::remove_dir_all(path)?;
    Ok(())
}

pub fn move_to_recycle_bin(path: &Path) -> Result<()> {
    #[cfg(all(target_os = "windows", feature = "windows"))]
    {
        greek_windows::move_to_recycle_bin(path)
    }

    #[cfg(not(all(target_os = "windows", feature = "windows")))]
    {
        // No recycle-bin integration available for this platform/feature set.
        // Fall back to a direct delete rather than silently succeeding.
        tracing::warn!(
            "Recycle bin not available, deleting directly: {}",
            path.display()
        );
        delete_directory(path)
    }
}

/// Delete a registry key (subkey) by its full path, e.g.
/// `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\Foo`.
///
/// On Windows this removes the subkey using the registry API. On other
/// platforms this is a no-op returning Ok (there is no registry to delete).
///
/// **Security:** refuses to delete any key whose path is a prefix-match
/// against `PROTECTED_REGISTRY_PATHS` to prevent accidental (or malicious)
/// deletion of critical OS registry entries.
pub fn delete_registry_key(path: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use greek_common::{GreekError, RegistryHive};
        use winreg::enums::*;
        use winreg::RegKey;

        let path = path.trim();
        if path.is_empty() {
            return Err(GreekError::RegistryError(
                "Empty registry key path".to_string(),
            ));
        }

        // ── V002: block deletion of protected registry paths ──────────
        if is_protected_registry_path(path) {
            return Err(GreekError::SafetyError(
                "Refusing to delete protected registry key: <redacted>".to_string(),
            ));
        }

        // Normalize hive prefix — accept HKLM, HKCU, HKCR, HKU with single backslash
        let trimmed = path.trim();
        let lower = trimmed.to_lowercase();
        let (hive, remainder): (RegistryHive, &str) = if lower.starts_with("hklm\\") {
            (RegistryHive::Hklm, &trimmed[5..])
        } else if lower.starts_with("hkcu\\") {
            (RegistryHive::Hkcu, &trimmed[5..])
        } else if lower.starts_with("hkcr\\") || lower.starts_with("hku\\") {
            return Err(GreekError::RegistryError(
                "HKCR/HKU hives not supported for deletion".to_string(),
            ));
        } else {
            return Err(GreekError::RegistryError(
                "Cannot determine registry hive from path (expected HKLM\\ or HKCU\\)".to_string(),
            ));
        };

        // The path is "Software\...\Uninstall\AppName"; the key to delete is
        // the leaf, and its parent is the rest.
        let (parent_path, leaf) = match remainder.rfind('\\') {
            Some(idx) => (&remainder[..idx], &remainder[idx + 1..]),
            None => {
                return Err(GreekError::RegistryError(
                    "Registry key path has no parent".to_string(),
                ));
            }
        };

        let root = match hive {
            RegistryHive::Hklm => HKEY_LOCAL_MACHINE,
            RegistryHive::Hkcu => HKEY_CURRENT_USER,
        };

        let parent = RegKey::predef(root).open_subkey(parent_path).map_err(|_| {
            GreekError::RegistryError("Failed to open registry parent key".to_string())
        })?;

        // Recursively delete the subkey.
        parent.delete_subkey_all(leaf).map_err(|_| {
            GreekError::RegistryError("Failed to delete registry subkey".to_string())
        })?;

        tracing::info!("Deleted registry key: <redacted>");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Ok(())
    }
}

pub fn get_app_data_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "reek", "reek-uninstaller").ok_or(
        GreekError::ConfigError("Failed to get project directories".to_string()),
    )?;

    Ok(proj_dirs.data_dir().to_path_buf())
}

pub fn get_config_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "reek", "reek-uninstaller").ok_or(
        GreekError::ConfigError("Failed to get project directories".to_string()),
    )?;

    Ok(proj_dirs.config_dir().to_path_buf())
}

pub fn get_cache_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "reek", "reek-uninstaller").ok_or(
        GreekError::ConfigError("Failed to get project directories".to_string()),
    )?;

    Ok(proj_dirs.cache_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1024), "1 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1 MiB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1 GiB");
    }

    #[test]
    fn test_is_protected_path() {
        let protected = vec!["C:\\Windows".to_string(), "C:\\Program Files".to_string()];

        assert!(is_protected_path(
            Path::new("C:\\Windows\\System32"),
            &protected
        ));
        assert!(is_protected_path(
            Path::new("C:\\Program Files\\App"),
            &protected
        ));
        assert!(!is_protected_path(Path::new("C:\\Users\\Test"), &protected));
    }

    #[test]
    fn test_is_protected_path_case_insensitive() {
        let protected = vec!["C:\\Windows".to_string()];

        assert!(is_protected_path(
            Path::new("c:\\windows\\system32"),
            &protected
        ));
        assert!(is_protected_path(
            Path::new("C:\\WINDOWS\\System32"),
            &protected
        ));
    }

    #[test]
    fn test_delete_directory_nonexistent_returns_error() {
        let result = delete_directory(Path::new("/nonexistent_path_reek_test"));
        assert!(result.is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_is_protected_registry_path() {
        assert!(is_protected_registry_path(
            "HKLM\\SYSTEM\\CurrentControlSet\\Services\\MyService"
        ));
        assert!(is_protected_registry_path(
            "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run"
        ));
        assert!(!is_protected_registry_path(
            "HKLM\\SOFTWARE\\MyCompany\\MyApp"
        ));
        assert!(!is_protected_registry_path(
            "HKCU\\SOFTWARE\\MyCompany\\MyApp"
        ));
    }

    #[test]
    fn test_ensure_directory_exists() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test").join("nested");

        assert!(!test_path.exists());
        ensure_directory_exists(&test_path).unwrap();
        assert!(test_path.exists());
    }

    #[test]
    fn test_get_file_size() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        std::fs::write(&test_file, b"Hello, World!").unwrap();

        let size = get_file_size(&test_file).unwrap();
        assert_eq!(size, 13);
    }
}
