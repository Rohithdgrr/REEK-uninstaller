// Windows Store/UWP app scanner implementation

use async_trait::async_trait;
use greek_common::{
    clean_publisher_name, AppScanner, GreekError, InstallSource, InstalledApp, Result,
};
use std::path::PathBuf;
use tracing::{info, warn};

/// Windows Store/UWP app scanner
pub struct WindowsStoreScanner {
    include_framework_apps: bool,
}

impl WindowsStoreScanner {
    pub fn new() -> Self {
        Self {
            include_framework_apps: false,
        }
    }

    pub fn with_framework_apps(include: bool) -> Self {
        Self {
            include_framework_apps: include,
        }
    }

    /// Scan for Windows Store/UWP apps using PowerShell
    pub async fn scan_store_apps(&self) -> Result<Vec<InstalledApp>> {
        info!("Scanning for Windows Store/UWP apps");

        let mut apps = Vec::new();

        // Use PowerShell to get Appx packages (compact query for speed)
        let ps_command = r#"
            Get-AppxPackage | Select-Object Name, PackageFullName, PackageFamilyName, 
            InstallLocation, Version, Publisher | 
            ConvertTo-Json -Depth 3
        "#;

        let output = self.run_powershell(ps_command).await?;

        if output.is_empty() {
            warn!("No output from PowerShell command");
            return Ok(apps);
        }

        // Parse JSON output
        let packages: Vec<serde_json::Value> = serde_json::from_str(&output).map_err(|e| {
            GreekError::SystemError(format!("Failed to parse PowerShell output: {}", e))
        })?;

        for package in packages {
            if let Some(app) = self.parse_appx_package(&package) {
                if !self.include_framework_apps && app.is_system_component {
                    continue;
                }
                apps.push(app);
            }
        }

        info!("Found {} Windows Store apps", apps.len());
        Ok(apps)
    }

    /// Parse an AppxPackage JSON object into an InstalledApp
    fn parse_appx_package(&self, package: &serde_json::Value) -> Option<InstalledApp> {
        let name = package.get("Name")?.as_str()?.to_string();
        let package_full_name = package.get("PackageFullName")?.as_str()?.to_string();
        let package_family_name = package.get("PackageFamilyName")?.as_str()?.to_string();
        let install_location = package.get("InstallLocation")?.as_str()?;
        let version = package
            .get("Version")
            .and_then(|v| v.as_str())
            .map(String::from);
        let publisher = package
            .get("Publisher")
            .and_then(|p| p.as_str())
            .map(clean_publisher_name);

        let is_framework =
            name.starts_with("Microsoft.") && (name.contains("UI") || name.contains("Framework"));

        let mut app = InstalledApp::new(
            name,
            InstallSource::WindowsStore {
                package_family_name,
                package_full_name,
            },
        );

        app.version = version;
        app.publisher = publisher;
        app.install_location = Some(PathBuf::from(install_location));
        app.is_system_component = is_framework;

        // Find main exe from install location (for process matching)
        let loc = PathBuf::from(install_location);
        if let Some(exe) = Self::find_exe_in_dir(&loc) {
            app.metadata.insert("exe_path".into(), exe);
        }

        // Package size from the install directory (falls back to 0 if inaccessible)
        if let Ok(size) = self.get_package_size(install_location) {
            app.size_bytes = (size > 0).then_some(size);
        }

        // Icon: extract from the package logo if available (best effort)
        let logo_candidates = ["Logo.png", "Square150x150Logo.png", "Square44x44Logo.png"];
        let loc = PathBuf::from(install_location);
        for logo in logo_candidates {
            let candidate = loc.join(logo);
            if candidate.exists() {
                app.metadata.insert(
                    "display_icon".into(),
                    candidate.to_string_lossy().into_owned(),
                );
                break;
            }
        }

        Some(app)
    }

    /// Get the size of an installed package
    fn get_package_size(&self, install_location: &str) -> Result<u64> {
        let path = PathBuf::from(install_location);
        if !path.exists() {
            return Ok(0);
        }

        // Calculate directory size recursively
        let mut total_size = 0;
        self.calculate_dir_size(&path, &mut total_size)?;
        Ok(total_size)
    }

    fn calculate_dir_size(&self, path: &PathBuf, total: &mut u64) -> Result<()> {
        if path.is_file() {
            if let Ok(metadata) = std::fs::metadata(path) {
                *total += metadata.len();
            }
        } else if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    self.calculate_dir_size(&entry.path(), total)?;
                }
            }
        }
        Ok(())
    }

    /// Run a PowerShell command and return output
    async fn run_powershell(&self, command: &str) -> Result<String> {
        let command = command.to_string();

        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", &command])
                .output()
        })
        .await
        .map_err(|e| GreekError::SystemError(format!("Task join error: {}", e)))?
        .map_err(|e| GreekError::SystemError(format!("Failed to execute PowerShell: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GreekError::SystemError(format!(
                "PowerShell command failed: {}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// Get package details including dependencies
    pub async fn get_package_details(
        &self,
        package_family_name: &str,
    ) -> Result<serde_json::Value> {
        let ps_command = format!(
            r#"
            Get-AppxPackage -PackageFamilyFilter '{}' | 
            Select-Object * | 
            ConvertTo-Json -Depth 5
            "#,
            package_family_name
        );

        let output = self.run_powershell(&ps_command).await?;

        serde_json::from_str(&output)
            .map_err(|e| GreekError::SystemError(format!("Failed to parse package details: {}", e)))
    }

    /// Remove a Windows Store app
    pub async fn remove_store_app(&self, package_family_name: &str) -> Result<()> {
        info!("Removing Windows Store app: {}", package_family_name);

        let ps_command = format!(
            r#"
            $package = Get-AppxPackage -PackageFamilyFilter '{}' -ErrorAction SilentlyContinue
            if ($package) {{
                Remove-AppxPackage -Package $package.PackageFullName -ErrorAction Stop
                Write-Output "Successfully removed"
            }} else {{
                Write-Output "Package not found"
            }}
            "#,
            package_family_name
        );

        let output = self.run_powershell(&ps_command).await?;

        if output.contains("Successfully removed") {
            info!(
                "Successfully removed Windows Store app: {}",
                package_family_name
            );
            Ok(())
        } else if output.contains("Package not found") {
            Err(GreekError::NotFound(format!(
                "Package '{}' not found",
                package_family_name
            )))
        } else {
            Err(GreekError::SystemError(format!(
                "Failed to remove package: {}",
                output
            )))
        }
    }

    /// Reset a Windows Store app
    pub async fn reset_store_app(&self, package_family_name: &str) -> Result<()> {
        info!("Resetting Windows Store app: {}", package_family_name);

        let ps_command = format!(
            r#"
            $package = Get-AppxPackage -PackageFamilyFilter '{}' -ErrorAction SilentlyContinue
            if ($package) {{
                Reset-AppxPackage -Package $package.PackageFullName -ErrorAction Stop
                Write-Output "Successfully reset"
            }} else {{
                Write-Output "Package not found"
            }}
            "#,
            package_family_name
        );

        let output = self.run_powershell(&ps_command).await?;

        if output.contains("Successfully reset") {
            info!(
                "Successfully reset Windows Store app: {}",
                package_family_name
            );
            Ok(())
        } else if output.contains("Package not found") {
            Err(GreekError::NotFound(format!(
                "Package '{}' not found",
                package_family_name
            )))
        } else {
            Err(GreekError::SystemError(format!(
                "Failed to reset package: {}",
                output
            )))
        }
    }

    /// Find the first .exe in a directory (1 level deep).
    fn find_exe_in_dir(dir: &PathBuf) -> Option<String> {
        use std::fs;
        let entries = fs::read_dir(dir).ok()?;
        let mut exes: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
            })
            .collect();
        exes.sort_by_key(|a| a.file_name());
        exes.into_iter()
            .next()
            .map(|e| e.path().to_string_lossy().into_owned())
    }
}

impl Default for WindowsStoreScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AppScanner for WindowsStoreScanner {
    fn scanner_id(&self) -> &'static str {
        "windows-store"
    }

    fn scanner_name(&self) -> String {
        "Windows Store/UWP Scanner".to_string()
    }

    async fn scan(&self) -> Result<Vec<InstalledApp>> {
        self.scan_store_apps().await
    }

    fn requires_elevation(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_scanner_creation() {
        let scanner = WindowsStoreScanner::new();
        assert!(!scanner.include_framework_apps);
    }

    #[tokio::test]
    async fn test_store_scanner_with_framework() {
        let scanner = WindowsStoreScanner::with_framework_apps(true);
        assert!(scanner.include_framework_apps);
    }

    #[tokio::test]
    async fn test_scan_store_apps() {
        let scanner = WindowsStoreScanner::new();
        // This test would only pass on Windows with installed Store apps
        let result = scanner.scan_store_apps().await;
        assert!(result.is_ok());
    }
}
