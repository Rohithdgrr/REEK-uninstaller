// Common platform-agnostic utilities

use greek_common::{GreekError, Result};
use std::path::PathBuf;

/// Get the current operating system
pub fn get_os() -> &'static str {
    std::env::consts::OS
}

/// Get the current architecture
pub fn get_arch() -> &'static str {
    std::env::consts::ARCH
}

/// Check if running with elevated privileges
pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Windows-specific elevation check
        false // Placeholder
    }

    #[cfg(unix)]
    {
        // Unix/Linux/macOS: check if running as root
        unsafe { libc::geteuid() == 0 }
    }
}

/// Get common application directories for the current platform
pub fn get_common_app_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            dirs.push(PathBuf::from(program_files));
        }
        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            dirs.push(PathBuf::from(program_files_x86));
        }
        if let Ok(program_data) = std::env::var("ProgramData") {
            dirs.push(PathBuf::from(program_data));
        }
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local_app_data));
        }
        if let Ok(app_data) = std::env::var("APPDATA") {
            dirs.push(PathBuf::from(app_data));
        }
    }

    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/share/applications"));
        dirs.push(PathBuf::from("/var/lib/applications"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".local/share/applications"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/Applications"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join("Applications"));
        }
    }

    dirs
}

/// Get the user's home directory
pub fn get_home_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(home) = std::env::var("USERPROFILE") {
            return Ok(PathBuf::from(home));
        }
        if let Ok(home_drive) = std::env::var("HOMEDRIVE") {
            if let Ok(home_path) = std::env::var("HOMEPATH") {
                return Ok(PathBuf::from(home_drive).join(home_path));
            }
        }
    }

    #[cfg(unix)]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Ok(PathBuf::from(home));
        }
    }

    Err(GreekError::SystemError(
        "Could not determine home directory".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_os() {
        let os = get_os();
        assert!(!os.is_empty());
    }

    #[test]
    fn test_get_arch() {
        let arch = get_arch();
        assert!(!arch.is_empty());
    }

    #[test]
    fn test_get_common_app_dirs() {
        let dirs = get_common_app_dirs();
        assert!(!dirs.is_empty());
    }
}
