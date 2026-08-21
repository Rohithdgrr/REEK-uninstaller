// Windows registry scanner implementation

use async_trait::async_trait;
use greek_common::{
    clean_publisher_name, AppScanner, InstallSource, InstalledApp, RegistryHive, RegistryKey,
    RegistryValue, RegistryValueType, ScanError,
};
use std::path::PathBuf;
use winreg::enums::*;
use winreg::RegKey;

use crate::icon::poor_icon_source;

const UNINSTALL_PATH_NATIVE: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL_PATH_WOW64: &str =
    r"Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

/// Windows registry scanner for installed applications
pub struct WindowsRegistryScanner {
    scan_64bit: bool,
    scan_32bit: bool,
    include_system_components: bool,
}

impl WindowsRegistryScanner {
    pub fn new() -> Self {
        Self {
            scan_64bit: true,
            scan_32bit: true,
            include_system_components: false,
        }
    }

    pub fn with_options(
        scan_64bit: bool,
        scan_32bit: bool,
        include_system_components: bool,
    ) -> Self {
        Self {
            scan_64bit,
            scan_32bit,
            include_system_components,
        }
    }

    fn scan_hive(
        &self,
        hive: RegistryHive,
        is_64bit: bool,
    ) -> Result<Vec<InstalledApp>, ScanError> {
        let mut apps = Vec::new();

        let uninstall_path = if is_64bit {
            UNINSTALL_PATH_NATIVE
        } else {
            UNINSTALL_PATH_WOW64
        };
        let root_key = match hive {
            RegistryHive::Hklm => HKEY_LOCAL_MACHINE,
            RegistryHive::Hkcu => HKEY_CURRENT_USER,
        };

        let uninstall_key = match RegKey::predef(root_key).open_subkey(uninstall_path) {
            Ok(key) => key,
            Err(e) => {
                tracing::warn!("Failed to open uninstall key for {:?}: {}", hive, e);
                return Ok(apps);
            }
        };

        for subkey_name in uninstall_key.enum_keys() {
            let subkey_name = match subkey_name {
                Ok(name) => name,
                Err(e) => {
                    tracing::warn!("Failed to enumerate subkey: {}", e);
                    continue;
                }
            };

            let app_key = match uninstall_key.open_subkey(&subkey_name) {
                Ok(key) => key,
                Err(e) => {
                    tracing::warn!("Failed to open subkey {}: {}", subkey_name, e);
                    continue;
                }
            };

            if let Some(app) = self.parse_registry_entry(&app_key, &subkey_name, hive, is_64bit) {
                if self.include_system_components || !app.is_system_component {
                    apps.push(app);
                }
            }
        }

        Ok(apps)
    }

    fn parse_registry_entry(
        &self,
        key: &RegKey,
        subkey_name: &str,
        hive: RegistryHive,
        is_64bit: bool,
    ) -> Option<InstalledApp> {
        // Get display name
        let display_name: Result<String, _> = key.get_value("DisplayName");
        let name = display_name.ok()?;

        // Skip empty names
        if name.trim().is_empty() {
            return None;
        }

        // Skip Windows Update entries if not including system components
        if !self.include_system_components && subkey_name.starts_with("KB") {
            return None;
        }

        let mut app = InstalledApp::new(
            name,
            InstallSource::Registry {
                hive,
                key_path: format!(
                    "{}\\{}",
                    if is_64bit {
                        UNINSTALL_PATH_NATIVE
                    } else {
                        UNINSTALL_PATH_WOW64
                    },
                    subkey_name
                ),
            },
        );

        // Get optional values
        app.publisher = key
            .get_value::<String, _>("Publisher")
            .ok()
            .map(|p| clean_publisher_name(&p));
        app.version = key.get_value::<String, _>("DisplayVersion").ok();
        app.install_location = key
            .get_value::<String, _>("InstallLocation")
            .ok()
            .map(PathBuf::from);
        app.uninstall_string = key.get_value::<String, _>("UninstallString").ok();
        app.quiet_uninstall_string = key.get_value::<String, _>("QuietUninstallString").ok();
        app.modify_string = key.get_value::<String, _>("ModifyPath").ok();

        // Parse install date
        if let Ok(date_str) = key.get_value::<String, _>("InstallDate") {
            app.install_date = self.parse_install_date(&date_str);
        }

        // Parse size
        if let Ok(size_str) = key.get_value::<String, _>("EstimatedSize") {
            app.size_bytes = size_str.parse::<u64>().ok().map(|kb| kb * 1024);
        }

        // Check if system component
        app.is_system_component = key
            .get_value::<u32, _>("SystemComponent")
            .ok()
            .map(|v| v == 1)
            .unwrap_or(false);

        // Store registry key information
        let registry_key = self.extract_registry_key(key, subkey_name, hive);
        app.registry_keys.push(registry_key);

        // Add metadata
        app.metadata
            .insert("registry_key".to_string(), subkey_name.to_string());
        app.metadata
            .insert("hive".to_string(), hive.as_str().to_string());

        // Icon source: DisplayIcon registry value
        if let Ok(icon) = key.get_value::<String, _>("DisplayIcon") {
            app.metadata.insert("display_icon".to_string(), icon);
        }

        // Best-effort exe path derived from the uninstall string (for icon extraction)
        let uninstall = app
            .uninstall_string
            .clone()
            .or_else(|| app.quiet_uninstall_string.clone())
            .or_else(|| key.get_value::<String, _>("ModifyPath").ok());
        if let Some(cmd) = uninstall {
            if let Some(exe) = Self::extract_exe_from_command(&cmd) {
                app.metadata.insert("exe_path".to_string(), exe);
            }
        }

        // Fallback: find an exe in the install location
        if !app.metadata.contains_key("exe_path") {
            if let Some(loc) = &app.install_location {
                if let Some(exe) = Self::find_exe_in_dir(loc) {
                    app.metadata.insert("exe_path".to_string(), exe);
                }
            }
        }

        Some(app)
    }

    /// Find the first plausible .exe in a directory (1 level deep).
    /// Generic installer binaries (unins*, setup*, msiexec, ...) are skipped:
    /// they carry no useful icon and are never the app's running process.
    fn find_exe_in_dir(dir: &PathBuf) -> Option<String> {
        use std::fs;
        let entries = fs::read_dir(dir).ok()?;
        let mut exes: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
                    && !poor_icon_source(&e.path())
            })
            .collect();
        exes.sort_by_key(|a| a.file_name());
        exes.into_iter()
            .next()
            .map(|e| e.path().to_string_lossy().into_owned())
    }

    /// Pull the first .exe/.msi path out of a (possibly quoted) command line.
    fn extract_exe_from_command(cmd: &str) -> Option<String> {
        let t = cmd.trim();
        if t.is_empty() {
            return None;
        }
        let inner = if let Some(rest) = t.strip_prefix('"') {
            rest.split('"').next().unwrap_or("").trim().to_string()
        } else {
            t.split_whitespace().next().unwrap_or("").to_string()
        };
        if inner.is_empty() {
            return None;
        }
        let ext = PathBuf::from(&inner)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase());
        match ext.as_deref() {
            Some("exe") | Some("msi") | Some("dll") => Some(inner),
            _ => None,
        }
    }

    fn parse_install_date(&self, date_str: &str) -> Option<chrono::NaiveDate> {
        // Windows stores dates as YYYYMMDD
        if date_str.len() >= 8 {
            let year = date_str[0..4].parse::<i32>().ok()?;
            let month = date_str[4..6].parse::<u32>().ok()?;
            let day = date_str[6..8].parse::<u32>().ok()?;

            chrono::NaiveDate::from_ymd_opt(year, month, day)
        } else {
            None
        }
    }

    fn extract_registry_key(
        &self,
        key: &RegKey,
        subkey_name: &str,
        hive: RegistryHive,
    ) -> RegistryKey {
        let mut registry_key = RegistryKey {
            path: format!("{}\\{}", hive.as_str(), subkey_name),
            hive,
            values: std::collections::HashMap::new(),
        };

        // Extract some common values
        let common_values = vec![
            "DisplayName",
            "DisplayVersion",
            "Publisher",
            "InstallLocation",
            "UninstallString",
            "QuietUninstallString",
            "InstallDate",
        ];

        for value_name in common_values {
            if let Ok(value) = key.get_raw_value(value_name) {
                let reg_value = self.convert_winreg_value(&value);
                registry_key
                    .values
                    .insert(value_name.to_string(), reg_value);
            }
        }

        registry_key
    }

    fn convert_winreg_value(&self, value: &winreg::RegValue) -> RegistryValue {
        use winreg::enums::RegType;

        let value_type = match value.vtype {
            RegType::REG_SZ => RegistryValueType::Sz,
            RegType::REG_EXPAND_SZ => RegistryValueType::ExpandSz,
            RegType::REG_BINARY => RegistryValueType::Binary,
            RegType::REG_DWORD => RegistryValueType::Dword,
            RegType::REG_DWORD_BIG_ENDIAN => RegistryValueType::DwordBigEndian,
            RegType::REG_LINK => RegistryValueType::Link,
            RegType::REG_MULTI_SZ => RegistryValueType::MultiSz,
            RegType::REG_QWORD => RegistryValueType::Qword,
            RegType::REG_NONE => RegistryValueType::None,
            _ => RegistryValueType::None,
        };

        let data = if let Ok(s) = String::from_utf8(value.bytes.clone()) {
            s
        } else {
            format!("{:?}", value.bytes)
        };

        RegistryValue { value_type, data }
    }
}

impl Default for WindowsRegistryScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AppScanner for WindowsRegistryScanner {
    fn scanner_id(&self) -> &'static str {
        "windows-registry"
    }

    fn scanner_name(&self) -> String {
        "Windows Registry Scanner".to_string()
    }

    async fn scan(&self) -> Result<Vec<InstalledApp>, greek_common::GreekError> {
        tracing::info!("Starting Windows registry scan");

        let scan_64bit = self.scan_64bit;
        let scan_32bit = self.scan_32bit;
        let include_system = self.include_system_components;

        let all_apps = tokio::task::spawn_blocking(move || {
            let scanner =
                WindowsRegistryScanner::with_options(scan_64bit, scan_32bit, include_system);
            let mut all_apps = Vec::new();

            // Scan HKLM (system-wide) 64-bit
            match scanner.scan_hive(RegistryHive::Hklm, true) {
                Ok(apps) => {
                    tracing::info!("Found {} apps in HKLM 64-bit", apps.len());
                    all_apps.extend(apps);
                }
                Err(e) => tracing::error!("Failed to scan HKLM 64-bit: {}", e),
            }

            // Scan HKLM 32-bit
            match scanner.scan_hive(RegistryHive::Hklm, false) {
                Ok(apps) => {
                    tracing::info!("Found {} apps in HKLM 32-bit", apps.len());
                    all_apps.extend(apps);
                }
                Err(e) => tracing::error!("Failed to scan HKLM 32-bit: {}", e),
            }

            // Scan HKCU (user-specific) 64-bit
            match scanner.scan_hive(RegistryHive::Hkcu, true) {
                Ok(apps) => {
                    tracing::info!("Found {} apps in HKCU 64-bit", apps.len());
                    all_apps.extend(apps);
                }
                Err(e) => tracing::error!("Failed to scan HKCU 64-bit: {}", e),
            }

            // Scan HKCU 32-bit
            match scanner.scan_hive(RegistryHive::Hkcu, false) {
                Ok(apps) => {
                    tracing::info!("Found {} apps in HKCU 32-bit", apps.len());
                    all_apps.extend(apps);
                }
                Err(e) => tracing::error!("Failed to scan HKCU 32-bit: {}", e),
            }

            all_apps
        })
        .await
        .map_err(|e| greek_common::GreekError::ScanError(format!("Task join error: {}", e)))?;

        tracing::info!(
            "Windows registry scan completed, total apps: {}",
            all_apps.len()
        );

        Ok(all_apps)
    }

    fn requires_elevation(&self) -> bool {
        true // HKLM requires elevation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let scanner = WindowsRegistryScanner::new();
        assert!(scanner.scan_64bit);
        assert!(scanner.scan_32bit);
        assert!(!scanner.include_system_components);
    }

    #[test]
    fn test_date_parsing() {
        let scanner = WindowsRegistryScanner::new();

        // Valid date
        let date = scanner.parse_install_date("20240315");
        assert!(date.is_some());
        assert_eq!(
            date.unwrap(),
            chrono::NaiveDate::from_ymd_opt(2024, 3, 15).unwrap()
        );

        // Invalid date
        let date = scanner.parse_install_date("invalid");
        assert!(date.is_none());
    }
}
