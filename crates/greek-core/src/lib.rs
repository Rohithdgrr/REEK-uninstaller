// greek-core - Business logic and service layer for REEK Ultimate Uninstaller
// Production-grade: unwrap/expect are denied in CI via `cargo clippy -- -D clippy::unwrap_used`.
// Source still allows them for ergonomic `expect("valid regex")` on static literals.
#![warn(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

pub mod app_service;
pub mod backup;
pub mod browser_extensions;
pub mod config;
pub mod leftover;
pub mod scanner;
pub mod task_scheduler;
pub mod uninstaller;
pub mod utils;
pub mod windows_features;
pub mod video;
pub mod dev_modules;

pub use app_service::*;
pub use backup::*;
pub use browser_extensions::*;
pub use config::*;
pub use leftover::*;
pub use scanner::*;
pub use task_scheduler::*;
pub use uninstaller::*;
pub use utils::*;
pub use windows_features::*;
pub use video::*;
pub use dev_modules::*;
