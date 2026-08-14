// Utility functions for REEK Ultimate Uninstaller

use greek_common::{GreekError, Result};
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use humansize::{format_size, BINARY};

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

pub fn is_protected_path(path: &Path, protected_paths: &[String]) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    
    for protected in protected_paths {
        let protected_lower = protected.to_lowercase();
        if path_str.starts_with(&protected_lower) {
            return true;
        }
    }
    
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
    std::fs::remove_dir_all(path)?;
    Ok(())
}

pub fn move_to_recycle_bin(path: &Path) -> Result<()> {
    // Platform-specific implementation
    #[cfg(target_os = "windows")]
    {
        // Windows-specific recycle bin implementation
        // This would use Windows API to move to recycle bin
        tracing::warn!("Recycle bin not implemented yet, deleting directly");
        delete_directory(path)?;
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux-specific implementation (trash-cli or similar)
        tracing::warn!("Recycle bin not implemented yet, deleting directly");
        delete_directory(path)?;
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS-specific implementation
        tracing::warn!("Recycle bin not implemented yet, deleting directly");
        delete_directory(path)?;
    }
    
    Ok(())
}

pub fn get_app_data_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "reek", "reek-uninstaller")
        .ok_or(GreekError::ConfigError("Failed to get project directories".to_string()))?;
    
    Ok(proj_dirs.data_dir().to_path_buf())
}

pub fn get_config_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "reek", "reek-uninstaller")
        .ok_or(GreekError::ConfigError("Failed to get project directories".to_string()))?;
    
    Ok(proj_dirs.config_dir().to_path_buf())
}

pub fn get_cache_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "reek", "reek-uninstaller")
        .ok_or(GreekError::ConfigError("Failed to get project directories".to_string()))?;
    
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
        let protected = vec![
            "C:\\Windows".to_string(),
            "C:\\Program Files".to_string(),
        ];
        
        assert!(is_protected_path(Path::new("C:\\Windows\\System32"), &protected));
        assert!(is_protected_path(Path::new("C:\\Program Files\\App"), &protected));
        assert!(!is_protected_path(Path::new("C:\\Users\\Test"), &protected));
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
