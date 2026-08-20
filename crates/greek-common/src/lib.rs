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
/// Uses **case-insensitive prefix matching**, so `C:\\Windows\\System32` is
/// correctly blocked even if only `C:\\Windows` appears in the protected list.
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
