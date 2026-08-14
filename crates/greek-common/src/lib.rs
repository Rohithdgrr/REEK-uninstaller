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

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "reek-uninstaller";
