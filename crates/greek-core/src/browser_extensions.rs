// Browser extension scanner for Chrome, Firefox, Edge, and Opera

use greek_common::{BrowserType, GreekError, InstallSource, InstalledApp, Result};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Browser extension information
#[derive(Debug, Clone)]
pub struct BrowserExtension {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub browser: BrowserType,
    pub enabled: bool,
    pub install_time: Option<String>,
    pub path: PathBuf,
}

/// Browser extension scanner
pub struct BrowserExtensionScanner {
    scan_browsers: Vec<BrowserType>,
}

impl BrowserExtensionScanner {
    pub fn new() -> Self {
        Self {
            scan_browsers: vec![
                BrowserType::Chrome,
                BrowserType::Firefox,
                BrowserType::Edge,
                BrowserType::Opera,
            ],
        }
    }

    pub fn with_browsers(browsers: Vec<BrowserType>) -> Self {
        Self {
            scan_browsers: browsers,
        }
    }

    /// Scan for browser extensions across all supported browsers
    pub async fn scan_extensions(&self) -> Result<Vec<BrowserExtension>> {
        info!("Scanning for browser extensions");

        let mut extensions = Vec::new();

        for browser in &self.scan_browsers {
            match browser {
                BrowserType::Chrome => {
                    if let Ok(chrome_exts) = self.scan_chrome_extensions().await {
                        extensions.extend(chrome_exts);
                    }
                }
                BrowserType::Firefox => {
                    if let Ok(firefox_exts) = self.scan_firefox_extensions().await {
                        extensions.extend(firefox_exts);
                    }
                }
                BrowserType::Edge => {
                    if let Ok(edge_exts) = self.scan_edge_extensions().await {
                        extensions.extend(edge_exts);
                    }
                }
                BrowserType::Opera => {
                    if let Ok(opera_exts) = self.scan_opera_extensions().await {
                        extensions.extend(opera_exts);
                    }
                }
                _ => {
                    warn!("Browser {:?} not supported for extension scanning", browser);
                }
            }
        }

        info!("Found {} browser extensions", extensions.len());
        Ok(extensions)
    }

    /// Get the extension directory for a browser
    fn get_extension_dir(&self, browser: &BrowserType) -> Option<PathBuf> {
        match browser {
            BrowserType::Chrome => {
                #[cfg(target_os = "windows")]
                {
                    Some(
                        PathBuf::from(std::env::var("LOCALAPPDATA").ok()?)
                            .join("Google")
                            .join("Chrome")
                            .join("User Data")
                            .join("Default")
                            .join("Extensions"),
                    )
                }
                #[cfg(target_os = "linux")]
                {
                    Some(
                        PathBuf::from(&_home)
                            .join(".config")
                            .join("google-chrome")
                            .join("Default")
                            .join("Extensions"),
                    )
                }
                #[cfg(target_os = "macos")]
                {
                    Some(
                        PathBuf::from(&_home)
                            .join("Library")
                            .join("Application Support")
                            .join("Google")
                            .join("Chrome")
                            .join("Default")
                            .join("Extensions"),
                    )
                }
                #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
                {
                    None
                }
            }
            BrowserType::Firefox => {
                #[cfg(target_os = "windows")]
                {
                    Some(
                        PathBuf::from(std::env::var("APPDATA").ok()?)
                            .join("Mozilla")
                            .join("Firefox")
                            .join("Profiles"),
                    )
                }
                #[cfg(target_os = "linux")]
                {
                    Some(PathBuf::from(&_home).join(".mozilla").join("firefox"))
                }
                #[cfg(target_os = "macos")]
                {
                    Some(
                        PathBuf::from(&_home)
                            .join("Library")
                            .join("Application Support")
                            .join("Firefox")
                            .join("Profiles"),
                    )
                }
                #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
                {
                    None
                }
            }
            BrowserType::Edge => {
                #[cfg(target_os = "windows")]
                {
                    Some(
                        PathBuf::from(std::env::var("LOCALAPPDATA").ok()?)
                            .join("Microsoft")
                            .join("Edge")
                            .join("User Data")
                            .join("Default")
                            .join("Extensions"),
                    )
                }
                #[cfg(target_os = "linux")]
                {
                    Some(
                        PathBuf::from(&_home)
                            .join(".config")
                            .join("microsoft-edge")
                            .join("Default")
                            .join("Extensions"),
                    )
                }
                #[cfg(target_os = "macos")]
                {
                    Some(
                        PathBuf::from(&_home)
                            .join("Library")
                            .join("Application Support")
                            .join("Microsoft Edge")
                            .join("Default")
                            .join("Extensions"),
                    )
                }
                #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
                {
                    None
                }
            }
            BrowserType::Opera => {
                #[cfg(target_os = "windows")]
                {
                    Some(
                        PathBuf::from(std::env::var("APPDATA").ok()?)
                            .join("Opera Software")
                            .join("Opera Stable")
                            .join("Extensions"),
                    )
                }
                #[cfg(target_os = "linux")]
                {
                    Some(
                        PathBuf::from(&_home)
                            .join(".config")
                            .join("opera")
                            .join("Extensions"),
                    )
                }
                #[cfg(target_os = "macos")]
                {
                    Some(
                        PathBuf::from(&_home)
                            .join("Library")
                            .join("Application Support")
                            .join("com.operasoftware.Opera")
                            .join("Extensions"),
                    )
                }
                #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
                {
                    None
                }
            }
            _ => None,
        }
    }

    /// Scan Chrome extensions
    async fn scan_chrome_extensions(&self) -> Result<Vec<BrowserExtension>> {
        let ext_dir = self
            .get_extension_dir(&BrowserType::Chrome)
            .ok_or_else(|| {
                GreekError::SystemError("Chrome extensions directory not found".to_string())
            })?;

        self.scan_chromium_extensions(&ext_dir, BrowserType::Chrome)
            .await
    }

    /// Scan Edge extensions (Chromium-based)
    async fn scan_edge_extensions(&self) -> Result<Vec<BrowserExtension>> {
        let ext_dir = self.get_extension_dir(&BrowserType::Edge).ok_or_else(|| {
            GreekError::SystemError("Edge extensions directory not found".to_string())
        })?;

        self.scan_chromium_extensions(&ext_dir, BrowserType::Edge)
            .await
    }

    /// Scan Opera extensions (Chromium-based)
    async fn scan_opera_extensions(&self) -> Result<Vec<BrowserExtension>> {
        let ext_dir = self.get_extension_dir(&BrowserType::Opera).ok_or_else(|| {
            GreekError::SystemError("Opera extensions directory not found".to_string())
        })?;

        self.scan_chromium_extensions(&ext_dir, BrowserType::Opera)
            .await
    }

    /// Scan Chromium-based browser extensions
    async fn scan_chromium_extensions(
        &self,
        ext_dir: &PathBuf,
        browser: BrowserType,
    ) -> Result<Vec<BrowserExtension>> {
        let mut extensions = Vec::new();

        if !ext_dir.exists() {
            return Ok(extensions);
        }

        let entries = std::fs::read_dir(ext_dir).map_err(|e| {
            GreekError::ScanError(format!("Failed to read extensions directory: {}", e))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Each extension directory contains version directories
                if let Ok(version_dirs) = std::fs::read_dir(&path) {
                    for version_dir in version_dirs.flatten() {
                        let version_path = version_dir.path();
                        if version_path.is_dir() {
                            if let Some(ext) =
                                self.parse_chromium_extension(&version_path, &browser).await
                            {
                                extensions.push(ext);
                            }
                        }
                    }
                }
            }
        }

        Ok(extensions)
    }

    /// Parse a Chromium extension manifest
    async fn parse_chromium_extension(
        &self,
        path: &Path,
        browser: &BrowserType,
    ) -> Option<BrowserExtension> {
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            return None;
        }

        let manifest_content = std::fs::read_to_string(&manifest_path).ok()?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_content).ok()?;

        let id = path.parent()?.file_name()?.to_str()?.to_string();

        let name = manifest
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown Extension")
            .to_string();

        let version = manifest
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let description = manifest
            .get("description")
            .and_then(|d| d.as_str())
            .map(String::from);

        Some(BrowserExtension {
            id,
            name,
            version,
            description,
            browser: *browser,
            enabled: true, // Would need to check Preferences file
            install_time: None,
            path: path.to_path_buf(),
        })
    }

    /// Scan Firefox extensions
    async fn scan_firefox_extensions(&self) -> Result<Vec<BrowserExtension>> {
        let ext_dir = self
            .get_extension_dir(&BrowserType::Firefox)
            .ok_or_else(|| {
                GreekError::SystemError("Firefox extensions directory not found".to_string())
            })?;

        let mut extensions = Vec::new();

        if !ext_dir.exists() {
            return Ok(extensions);
        }

        // Firefox profiles contain extensions
        let entries = std::fs::read_dir(&ext_dir).map_err(|e| {
            GreekError::ScanError(format!("Failed to read Firefox profiles: {}", e))
        })?;

        for entry in entries.flatten() {
            let profile_path = entry.path();
            if profile_path.is_dir() {
                let extensions_json = profile_path.join("extensions.json");
                if extensions_json.exists() {
                    if let Ok(profile_exts) = self
                        .parse_firefox_extensions_json(&extensions_json, &profile_path)
                        .await
                    {
                        extensions.extend(profile_exts);
                    }
                }
            }
        }

        Ok(extensions)
    }

    /// Parse Firefox extensions.json
    async fn parse_firefox_extensions_json(
        &self,
        json_path: &Path,
        profile_path: &Path,
    ) -> Result<Vec<BrowserExtension>> {
        let content = std::fs::read_to_string(json_path).map_err(|e| {
            GreekError::SystemError(format!("Failed to read extensions.json: {}", e))
        })?;

        let data: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            GreekError::SystemError(format!("Failed to parse extensions.json: {}", e))
        })?;

        let mut extensions = Vec::new();

        if let Some(addons) = data.get("addons").and_then(|a| a.as_array()) {
            for addon in addons {
                if let Some(ext) = self.parse_firefox_addon(addon, profile_path) {
                    extensions.push(ext);
                }
            }
        }

        Ok(extensions)
    }

    /// Parse a Firefox addon entry
    fn parse_firefox_addon(
        &self,
        addon: &serde_json::Value,
        profile_path: &Path,
    ) -> Option<BrowserExtension> {
        let id = addon.get("id")?.as_str()?.to_string();
        let name = addon
            .get("defaultLocale")
            .and_then(|l| l.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let version = addon
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let description = addon
            .get("defaultLocale")
            .and_then(|l| l.get("description"))
            .and_then(|d| d.as_str())
            .map(String::from);

        let enabled = addon
            .get("active")
            .and_then(|a| a.as_bool())
            .unwrap_or(false);

        let install_time = addon
            .get("installDate")
            .and_then(|t| t.as_str())
            .map(String::from);

        // Firefox stores extensions in a profile-specific way
        let ext_path = profile_path.join("extensions").join(format!("{}.xpi", id));

        Some(BrowserExtension {
            id,
            name,
            version,
            description,
            browser: BrowserType::Firefox,
            enabled,
            install_time,
            path: ext_path,
        })
    }

    /// Convert browser extension to InstalledApp
    pub fn extension_to_app(&self, ext: &BrowserExtension) -> InstalledApp {
        let browser_name = match ext.browser {
            BrowserType::Chrome => "Chrome",
            BrowserType::Firefox => "Firefox",
            BrowserType::Edge => "Edge",
            BrowserType::Opera => "Opera",
            BrowserType::Safari => "Safari",
        };

        let mut app = InstalledApp::new(
            format!("{} Extension: {}", browser_name, ext.name),
            InstallSource::BrowserExtension {
                browser: ext.browser,
                extension_id: ext.id.clone(),
            },
        );

        app.version = Some(ext.version.clone());
        app.publisher = ext.description.clone();
        app.install_location = Some(ext.path.clone());
        app.is_system_component = false;

        app
    }

    /// Find extensions related to a specific application
    pub async fn find_extensions_for_app(
        &self,
        app: &InstalledApp,
    ) -> Result<Vec<BrowserExtension>> {
        let all_extensions = self.scan_extensions().await?;

        let app_name_lower = app.name.to_lowercase();
        let publisher_lower = app.publisher.as_ref().map(|p| p.to_lowercase());

        let matching: Vec<BrowserExtension> = all_extensions
            .into_iter()
            .filter(|ext| {
                let name_match = ext.name.to_lowercase().contains(&app_name_lower);
                let desc_match = ext
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&app_name_lower))
                    .unwrap_or(false);
                let publisher_match = publisher_lower
                    .as_ref()
                    .map(|p| {
                        ext.name.to_lowercase().contains(p)
                            || ext
                                .description
                                .as_ref()
                                .map(|d| d.to_lowercase().contains(p))
                                .unwrap_or(false)
                    })
                    .unwrap_or(false);

                name_match || desc_match || publisher_match
            })
            .collect();

        info!("Found {} extensions for app '{}'", matching.len(), app.name);
        Ok(matching)
    }
}

impl Default for BrowserExtensionScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scanner_creation() {
        let scanner = BrowserExtensionScanner::new();
        assert!(!scanner.scan_browsers.is_empty());
    }

    #[tokio::test]
    async fn test_scan_extensions() {
        let scanner = BrowserExtensionScanner::new();
        let result = scanner.scan_extensions().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_extension_to_app() {
        let scanner = BrowserExtensionScanner::new();
        let ext = BrowserExtension {
            id: "test-id".to_string(),
            name: "Test Extension".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A test extension".to_string()),
            browser: BrowserType::Chrome,
            enabled: true,
            install_time: None,
            path: PathBuf::from("/test"),
        };

        let app = scanner.extension_to_app(&ext);
        assert!(app.name.contains("Test Extension"));
        assert!(app.name.contains("Chrome"));
    }
}
