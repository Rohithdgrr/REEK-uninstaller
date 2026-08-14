// greek-windows - Windows-specific implementations for REEK Ultimate Uninstaller

#[cfg(target_os = "windows")]
pub mod registry;
#[cfg(target_os = "windows")]
pub mod services;
#[cfg(target_os = "windows")]
pub mod restore;
#[cfg(target_os = "windows")]
pub mod wmi;
#[cfg(target_os = "windows")]
pub mod store;
#[cfg(target_os = "windows")]
pub mod icon;
#[cfg(target_os = "windows")]
pub mod system_stats;

#[cfg(target_os = "windows")]
pub use registry::*;
#[cfg(target_os = "windows")]
pub use services::*;
#[cfg(target_os = "windows")]
pub use restore::*;
#[cfg(target_os = "windows")]
pub use wmi::*;
#[cfg(target_os = "windows")]
pub use store::*;
#[cfg(target_os = "windows")]
pub use icon::*;
#[cfg(target_os = "windows")]
pub use system_stats::*;

// Stub implementations for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub mod registry {
    use greek_common::{InstalledApp, Result, GreekError};
    
    pub struct WindowsRegistryScanner;
    
    impl WindowsRegistryScanner {
        pub fn new() -> Self { Self }
    }
    
    impl Default for WindowsRegistryScanner {
        fn default() -> Self { Self::new() }
    }
    
    impl WindowsRegistryScanner {
        pub async fn scan(&self) -> Result<Vec<InstalledApp>> {
            Err(GreekError::SystemError("Registry scanning not available on this platform".to_string()))
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod services {
    use greek_common::{InstalledApp, Result, GreekError};
    
    pub struct WindowsServiceManager;
    
    impl WindowsServiceManager {
        pub fn new() -> Self { Self }
    }
    
    impl Default for WindowsServiceManager {
        fn default() -> Self { Self::new() }
    }
    
    impl WindowsServiceManager {
        pub async fn find_services_for_app(&self, _app: &InstalledApp) -> Result<Vec<String>> {
            Err(GreekError::SystemError("Service management not available on this platform".to_string()))
        }
        
        pub async fn stop_service(&self, _service_name: &str) -> Result<()> {
            Err(GreekError::SystemError("Service management not available on this platform".to_string()))
        }
        
        pub async fn delete_service(&self, _service_name: &str) -> Result<()> {
            Err(GreekError::SystemError("Service management not available on this platform".to_string()))
        }
        
        pub async fn cleanup_app_services(&self, _app: &InstalledApp) -> Result<Vec<String>> {
            Err(GreekError::SystemError("Service management not available on this platform".to_string()))
        }
        
        pub async fn list_all_services(&self) -> Result<Vec<String>> {
            Err(GreekError::SystemError("Service management not available on this platform".to_string()))
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod restore {
    use greek_common::{Result, GreekError};
    
    pub struct RestorePointManager;
    
    impl RestorePointManager {
        pub fn new() -> Self { Self }
    }
    
    impl Default for RestorePointManager {
        fn default() -> Self { Self::new() }
    }
    
    impl RestorePointManager {
        pub async fn create_restore_point(&self, _description: &str) -> Result<String> {
            Err(GreekError::SystemError("Restore points not available on this platform".to_string()))
        }
        
        pub async fn list_restore_points(&self) -> Result<Vec<String>> {
            Err(GreekError::SystemError("Restore points not available on this platform".to_string()))
        }
        
        pub async fn is_enabled(&self) -> Result<bool> {
            Ok(false)
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod wmi {
    use greek_common::{InstalledApp, Result, GreekError};
    
    pub struct WmiClient;
    
    impl WmiClient {
        pub fn new() -> Self { Self }
    }
    
    impl Default for WmiClient {
        fn default() -> Self { Self::new() }
    }
    
    impl WmiClient {
        pub async fn query_installed_software(&self) -> Result<Vec<InstalledApp>> {
            Err(GreekError::SystemError("WMI not available on this platform".to_string()))
        }
        
        pub async fn query_windows_features(&self) -> Result<Vec<String>> {
            Err(GreekError::SystemError("WMI not available on this platform".to_string()))
        }
        
        pub async fn query_processes(&self) -> Result<Vec<String>> {
            Err(GreekError::SystemError("WMI not available on this platform".to_string()))
        }
        
        pub async fn query_startup_items(&self) -> Result<Vec<String>> {
            Err(GreekError::SystemError("WMI not available on this platform".to_string()))
        }
        
        pub async fn query_scheduled_tasks(&self) -> Result<Vec<String>> {
            Err(GreekError::SystemError("WMI not available on this platform".to_string()))
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod store {
    use greek_common::{InstalledApp, Result, GreekError};
    
    pub struct WindowsStoreScanner;
    
    impl WindowsStoreScanner {
        pub fn new() -> Self { Self }
    }
    
    impl Default for WindowsStoreScanner {
        fn default() -> Self { Self::new() }
    }
    
    impl WindowsStoreScanner {
        pub async fn scan_store_apps(&self) -> Result<Vec<InstalledApp>> {
            Err(GreekError::SystemError("Windows Store scanning not available on this platform".to_string()))
        }
        
        pub async fn remove_store_app(&self, _package_family_name: &str) -> Result<()> {
            Err(GreekError::SystemError("Windows Store management not available on this platform".to_string()))
        }
        
        pub async fn reset_store_app(&self, _package_family_name: &str) -> Result<()> {
            Err(GreekError::SystemError("Windows Store management not available on this platform".to_string()))
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod system_stats {
    use std::collections::HashMap;

    #[derive(Debug, Clone, Default)]
    pub struct SystemStats {
        pub cpu_usage: f32,
        pub ram_used_bytes: u64,
        pub ram_total_bytes: u64,
        pub swap_used_bytes: u64,
        pub swap_total_bytes: u64,
        pub disks: Vec<DiskStat>,
        pub gpu: Option<GpuStat>,
        pub battery: Option<BatteryStat>,
        pub uptime_secs: u64,
        pub process_count: usize,
        pub processes: HashMap<String, ProcessUsage>,
    }

    #[derive(Debug, Clone, Default)]
    pub struct ProcessUsage {
        pub pid: u32,
        pub name: String,
        pub exe_path: String,
        pub cpu_usage: f32,
        pub memory_bytes: u64,
        pub virtual_memory: u64,
        pub run_time_secs: u64,
        pub started_at: Option<u64>,
        pub threads: usize,
        pub read_bytes: u64,
        pub written_bytes: u64,
        pub gpu_usage_pct: f32,
        pub vram_bytes: u64,
    }

    #[derive(Debug, Clone)]
    pub struct DiskStat {
        pub label: String,
        pub used_bytes: u64,
        pub total_bytes: u64,
    }

    #[derive(Debug, Clone)]
    pub struct GpuStat {
        pub name: String,
        pub usage_pct: f32,
        pub vram_used_bytes: u64,
        pub vram_total_bytes: u64,
    }

    #[derive(Debug, Clone)]
    pub struct BatteryStat {
        pub percent: u8,
        pub charging: bool,
    }

    pub struct SystemStatsCollector;

    impl SystemStatsCollector {
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for SystemStatsCollector {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SystemStatsCollector {
        pub fn collect(&mut self) -> SystemStats {
            SystemStats::default()
        }
    }
}