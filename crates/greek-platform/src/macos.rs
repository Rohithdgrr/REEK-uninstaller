// macOS-specific implementations for application management

use greek_common::{GreekError, InstallSource, InstalledApp, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

/// macOS .app bundle scanner
pub struct MacOsAppScanner {
    scan_directories: Vec<PathBuf>,
    include_user_apps: bool,
    include_system_apps: bool,
}

impl MacOsAppScanner {
    pub fn new() -> Self {
        let mut scan_dirs = Vec::new();

        // Common macOS application directories
        scan_dirs.push(PathBuf::from("/Applications"));

        if let Ok(home) = std::env::var("HOME") {
            scan_dirs.push(PathBuf::from(&home).join("Applications"));
        }

        Self {
            scan_directories: scan_dirs,
            include_user_apps: true,
            include_system_apps: true,
        }
    }

    pub fn with_directories(directories: Vec<PathBuf>) -> Self {
        Self {
            scan_directories: directories,
            include_user_apps: true,
            include_system_apps: true,
        }
    }

    pub fn include_user_apps(mut self, include: bool) -> Self {
        self.include_user_apps = include;
        self
    }

    pub fn include_system_apps(mut self, include: bool) -> Self {
        self.include_system_apps = include;
        self
    }

    /// Scan for installed applications
    pub async fn scan(&self) -> Result<Vec<InstalledApp>> {
        info!("Starting macOS application scan");

        let mut apps = Vec::new();

        for directory in &self.scan_directories {
            if !directory.exists() {
                warn!("Scan directory does not exist: {:?}", directory);
                continue;
            }

            let discovered = self.scan_directory(directory).await?;
            apps.extend(discovered);
        }

        // Also scan using system_profiler for more accurate data
        if let Ok(system_apps) = self.scan_system_profiler().await {
            apps.extend(system_apps);
        }

        // Scan LaunchAgents and LaunchDaemons
        if let Ok(_launch_items) = self.scan_launch_items().await {
            // These are typically not apps but related services
        }

        info!("macOS scan completed, found {} applications", apps.len());

        Ok(apps)
    }

    /// Scan a directory for .app bundles
    async fn scan_directory(&self, directory: &PathBuf) -> Result<Vec<InstalledApp>> {
        let mut apps = Vec::new();

        let entries = std::fs::read_dir(directory)
            .map_err(|e| GreekError::ScanError(format!("Failed to read directory: {}", e)))?;

        for entry in entries {
            let entry =
                entry.map_err(|e| GreekError::ScanError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            // Check if this is an .app bundle
            if path.is_dir() {
                if let Some(ext) = path.extension() {
                    if ext == "app" {
                        if let Some(app) = self.parse_app_bundle(&path).await? {
                            apps.push(app);
                        }
                    }
                }
            }
        }

        Ok(apps)
    }

    /// Parse an .app bundle and extract information
    async fn parse_app_bundle(&self, path: &Path) -> Result<Option<InstalledApp>> {
        let app_name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let mut app = InstalledApp::new(
            app_name.clone(),
            InstallSource::Portable {
                detected_path: path.to_path_buf(),
                confidence: 0.95,
            },
        );

        app.install_location = Some(path.to_path_buf());

        // Read Info.plist for detailed information
        let info_plist = path.join("Contents").join("Info.plist");
        if info_plist.exists() {
            match self.parse_info_plist(&info_plist).await {
                Ok(plist_info) => {
                    app.name = plist_info.name.unwrap_or(app_name);
                    app.version = plist_info.version;
                    app.publisher = plist_info.developer;
                    app.metadata.insert(
                        "bundle_id".to_string(),
                        plist_info.bundle_id.unwrap_or_default(),
                    );
                    app.metadata.insert(
                        "min_system_version".to_string(),
                        plist_info.min_os_version.unwrap_or_default(),
                    );
                }
                Err(e) => {
                    warn!("Failed to parse Info.plist for {}: {}", app_name, e);
                }
            }
        }

        // Get app size
        if let Ok(size) = self.get_app_size(path) {
            app.size_bytes = Some(size);
        }

        Ok(Some(app))
    }

    /// Parse Info.plist file
    async fn parse_info_plist(&self, plist_path: &PathBuf) -> Result<PlistInfo> {
        // Use plutil to convert plist to JSON for easier parsing
        let output = Command::new("plutil")
            .args(["-convert", "json", "-o", "-", &plist_path.to_string_lossy()])
            .output()
            .map_err(|e| GreekError::SystemError(format!("Failed to execute plutil: {}", e)))?;

        if !output.status.success() {
            // Fallback: try to read as XML plist
            return self.parse_xml_plist(plist_path).await;
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let plist: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| GreekError::SystemError(format!("Failed to parse plist JSON: {}", e)))?;

        Ok(PlistInfo {
            name: plist
                .get("CFBundleName")
                .and_then(|v| v.as_str())
                .map(String::from),
            version: plist
                .get("CFBundleShortVersionString")
                .and_then(|v| v.as_str())
                .map(String::from),
            bundle_id: plist
                .get("CFBundleIdentifier")
                .and_then(|v| v.as_str())
                .map(String::from),
            developer: plist
                .get("CFBundleDeveloperName")
                .and_then(|v| v.as_str())
                .map(String::from),
            min_os_version: plist
                .get("LSMinimumSystemVersion")
                .and_then(|v| v.as_str())
                .map(String::from),
        })
    }

    /// Parse XML plist as fallback
    async fn parse_xml_plist(&self, plist_path: &PathBuf) -> Result<PlistInfo> {
        let content = std::fs::read_to_string(plist_path)
            .map_err(|e| GreekError::SystemError(format!("Failed to read plist: {}", e)))?;

        // Simple XML parsing for common fields
        let name = self.extract_plist_string(&content, "CFBundleName");
        let version = self.extract_plist_string(&content, "CFBundleShortVersionString");
        let bundle_id = self.extract_plist_string(&content, "CFBundleIdentifier");
        let developer = self.extract_plist_string(&content, "CFBundleDeveloperName");
        let min_os = self.extract_plist_string(&content, "LSMinimumSystemVersion");

        Ok(PlistInfo {
            name,
            version,
            bundle_id,
            developer,
            min_os_version: min_os,
        })
    }

    /// Extract string value from XML plist content
    fn extract_plist_string(&self, content: &str, key: &str) -> Option<String> {
        let key_tag = format!("<key>{}</key>", key);
        if let Some(pos) = content.find(&key_tag) {
            let after_key = &content[pos + key_tag.len()..];
            if let Some(string_start) = after_key.find("<string>") {
                let string_content = &after_key[string_start + 8..];
                if let Some(string_end) = string_content.find("</string>") {
                    return Some(string_content[..string_end].to_string());
                }
            }
        }
        None
    }

    /// Get the size of an application bundle
    fn get_app_size(&self, path: &Path) -> Result<u64> {
        let output = Command::new("du")
            .args(["-sh", &path.to_string_lossy()])
            .output()
            .map_err(|e| GreekError::SystemError(format!("Failed to get app size: {}", e)))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let size_str = stdout.split_whitespace().next().unwrap_or("0");
            return self.parse_size_string(size_str);
        }

        Ok(0)
    }

    /// Parse size string like "1.2G", "500M", "100K"
    fn parse_size_string(&self, size_str: &str) -> Result<u64> {
        let size_str = size_str.trim().to_lowercase();

        if let Some(num_str) = size_str.strip_suffix('g') {
            let num: f64 = num_str.parse().unwrap_or(0.0);
            Ok((num * 1024.0 * 1024.0 * 1024.0) as u64)
        } else if let Some(num_str) = size_str.strip_suffix('m') {
            let num: f64 = num_str.parse().unwrap_or(0.0);
            Ok((num * 1024.0 * 1024.0) as u64)
        } else if let Some(num_str) = size_str.strip_suffix('k') {
            let num: f64 = num_str.parse().unwrap_or(0.0);
            Ok((num * 1024.0) as u64)
        } else {
            let num: u64 = size_str.parse().unwrap_or(0);
            Ok(num)
        }
    }

    /// Scan using system_profiler for additional information
    async fn scan_system_profiler(&self) -> Result<Vec<InstalledApp>> {
        let output = Command::new("system_profiler")
            .args(["SPApplicationsDataType", "-json"])
            .output()
            .map_err(|e| {
                GreekError::SystemError(format!("Failed to execute system_profiler: {}", e))
            })?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let data: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            GreekError::SystemError(format!("Failed to parse system_profiler output: {}", e))
        })?;

        let mut apps = Vec::new();

        if let Some(applications) = data.get("SPApplicationsDataType") {
            if let Some(apps_array) = applications.as_array() {
                for app_data in apps_array {
                    if let Some(app) = self.parse_system_profiler_app(app_data) {
                        apps.push(app);
                    }
                }
            }
        }

        Ok(apps)
    }

    /// Parse an application from system_profiler output
    fn parse_system_profiler_app(&self, data: &serde_json::Value) -> Option<InstalledApp> {
        let name = data.get("_name")?.as_str()?.to_string();
        let version = data
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from);
        let developer = data
            .get("developer")
            .and_then(|v| v.as_str())
            .map(String::from);
        let location = data
            .get("location")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);
        let size = data.get("size_bytes").and_then(|v| v.as_u64());

        let mut app = InstalledApp::new(
            name,
            InstallSource::Portable {
                detected_path: location.clone().unwrap_or_default(),
                confidence: 0.99,
            },
        );

        app.version = version;
        app.publisher = developer;
        app.install_location = location;
        app.size_bytes = size;

        Some(app)
    }

    /// Scan LaunchAgents and LaunchDaemons
    async fn scan_launch_items(&self) -> Result<Vec<LaunchItem>> {
        let mut items = Vec::new();

        let launch_dirs = vec![
            PathBuf::from("/Library/LaunchDaemons"),
            PathBuf::from("/Library/LaunchAgents"),
        ];

        if let Ok(home) = std::env::var("HOME") {
            let user_launch = PathBuf::from(&home).join("Library/LaunchAgents");
            if user_launch.exists() {
                items.extend(self.scan_launch_directory(&user_launch).await?);
            }
        }

        for dir in launch_dirs {
            if dir.exists() {
                items.extend(self.scan_launch_directory(&dir).await?);
            }
        }

        Ok(items)
    }

    /// Scan a LaunchAgents/LaunchDaemons directory
    async fn scan_launch_directory(&self, dir: &PathBuf) -> Result<Vec<LaunchItem>> {
        let mut items = Vec::new();

        let entries = std::fs::read_dir(dir).map_err(|e| {
            GreekError::ScanError(format!("Failed to read launch directory: {}", e))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "plist").unwrap_or(false) {
                if let Some(item) = self.parse_launch_plist(&path).await {
                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    /// Parse a launch plist file
    async fn parse_launch_plist(&self, path: &PathBuf) -> Option<LaunchItem> {
        let content = std::fs::read_to_string(path).ok()?;

        let label = self.extract_plist_string(&content, "Label")?;
        let program = self.extract_plist_string(&content, "Program");

        Some(LaunchItem {
            label,
            path: path.clone(),
            program,
            is_running: false, // Would need to check with launchctl
        })
    }

    /// Remove an application
    pub async fn remove_app(&self, app: &InstalledApp) -> Result<()> {
        if let Some(install_location) = &app.install_location {
            // Safety: refuse to remove protected system paths (CR-6)
            let protected = greek_common::PROTECTED_PATHS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            if greek_common::is_protected_path(install_location, &protected) {
                return Err(GreekError::SafetyError(format!(
                    "Refusing to remove protected path for app: {}",
                    app.name
                )));
            }

            info!(
                "Removing application: {:?} at {:?}",
                app.name, install_location
            );

            // Use rm -rf for the .app bundle
            let output = Command::new("sudo")
                .args(["rm", "-rf", &install_location.to_string_lossy()])
                .output()
                .map_err(|e| GreekError::SystemError(format!("Failed to remove app: {}", e)))?;

            if output.status.success() {
                info!("Successfully removed application: {}", app.name);
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(GreekError::UninstallError(format!(
                    "Failed to remove app: {}",
                    stderr
                )))
            }
        } else {
            Err(GreekError::SystemError(
                "No install location found for app".to_string(),
            ))
        }
    }
}

impl Default for MacOsAppScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Launch item information
#[derive(Debug, Clone)]
pub struct LaunchItem {
    pub label: String,
    pub path: PathBuf,
    pub program: Option<String>,
    pub is_running: bool,
}

/// Parsed Info.plist information
#[derive(Debug, Default)]
struct PlistInfo {
    name: Option<String>,
    version: Option<String>,
    bundle_id: Option<String>,
    developer: Option<String>,
    min_os_version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_macos_scanner_creation() {
        let scanner = MacOsAppScanner::new();
        assert!(!scanner.scan_directories.is_empty());
    }

    #[tokio::test]
    async fn test_parse_plist_string() {
        let scanner = MacOsAppScanner::new();
        let content = r#"
            <dict>
                <key>CFBundleName</key>
                <string>TestApp</string>
                <key>CFBundleVersion</key>
                <string>1.0.0</string>
            </dict>
        "#;

        assert_eq!(
            scanner.extract_plist_string(content, "CFBundleName"),
            Some("TestApp".to_string())
        );
        assert_eq!(
            scanner.extract_plist_string(content, "CFBundleVersion"),
            Some("1.0.0".to_string())
        );
        assert_eq!(scanner.extract_plist_string(content, "NonExistent"), None);
    }

    #[test]
    fn test_parse_size_string() {
        let scanner = MacOsAppScanner::new();

        assert_eq!(scanner.parse_size_string("1.5G").unwrap(), 1610612736);
        assert_eq!(scanner.parse_size_string("500M").unwrap(), 524288000);
        assert_eq!(scanner.parse_size_string("100K").unwrap(), 102400);
        assert_eq!(scanner.parse_size_string("1024").unwrap(), 1024);
    }
}
