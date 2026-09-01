// greek-common - Shared types, errors, and constants for the REEK Ultimate Uninstaller

pub mod constants;
pub mod error;
pub mod models;
pub mod traits;

#[cfg(test)]
mod tests;

pub use constants::*;
pub use error::*;
pub use models::*;
pub use traits::*;

use std::path::Path;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "reek-uninstaller";

/// Check whether a file path is under any of the given protected path prefixes.
///
/// Uses **case-insensitive, separator-aware prefix matching**:
/// `C:\Windows` blocks `C:\Windows\System32` but NOT `C:\WindowsAppsFoo`.
/// Normalizes `\` vs `/` and trailing separators.
pub fn is_protected_path(path: &Path, protected_paths: &[String]) -> bool {
    fn normalize(s: &str) -> String {
        let mut n = s.to_lowercase().replace('\\', "/");
        // trim trailing '/' (but keep root "/")
        while n.len() > 1 && n.ends_with('/') {
            n.pop();
        }
        n
    }

    let path_norm = normalize(&path.to_string_lossy());

    // Edge: path is exactly "/" — only blocked if "/" is explicitly protected
    // (we removed it from constants, but honor caller-provided list)
    for protected in protected_paths {
        let prot_norm = normalize(protected);
        if path_norm == prot_norm {
            return true;
        }
        // separator guard: must be prot + '/'
        if path_norm.starts_with(&(prot_norm.clone() + "/")) {
            return true;
        }
    }

    false
}
