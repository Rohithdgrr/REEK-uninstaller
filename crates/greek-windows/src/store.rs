// Windows Store/UWP app scanner implementation

use async_trait::async_trait;
use greek_common::{
    clean_publisher_name, AppScanner, GreekError, InstallSource, InstalledApp, Result,
};
use std::path::{Path, PathBuf};
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
        let is_os_store = greek_common::constants::is_os_critical_store_package(&name);

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
        app.is_system_component = is_framework || is_os_store;

        // Find main exe from install location (for process matching)
        let loc = PathBuf::from(install_location);
        if let Some(exe) = Self::find_exe_in_dir(&loc) {
            app.metadata.insert("exe_path".into(), exe);
        }

        // Package size from the install directory (falls back to 0 if inaccessible)
        if let Ok(size) = self.get_package_size(install_location) {
            app.size_bytes = (size > 0).then_some(size);
        }

        // Icon: extract from the package logo if available (best effort).
        // AppX packages store logos in subdirectories (e.g. Assets/) with
        // scale variants like "Square150x150Logo.scale-100.png".
        let loc = PathBuf::from(install_location);
        if let Some(logo) = Self::find_package_logo(&loc) {
            app.metadata.insert("display_icon".into(), logo);
        }

        Some(app)
    }

    /// Search a package directory (up to two levels deep, e.g. into an
    /// `Assets/` folder) for the best package logo PNG.
    ///
    /// Prefers Square150x150 over Square44x44 over any other `*Logo*` asset;
    /// within a family prefers plain / `scale-100` variants, then other scale
    /// factors, then ascending `targetsize-N` values.
    fn find_package_logo(dir: &Path) -> Option<String> {
        const MAX_DEPTH: usize = 2;

        fn rank(name_lower: &str) -> Option<(u8, u8, u32)> {
            let stem = name_lower.strip_suffix(".png")?;
            if !stem.contains("logo") {
                return None;
            }
            let family = if stem.starts_with("square150x150logo") {
                0u8
            } else if stem.starts_with("square44x44logo") {
                1
            } else {
                2
            };
            let (kind, size): (u8, u32) =
                if let Some(rest) = stem.split_once("targetsize-").map(|(_, r)| r) {
                    let n = rest
                        .split(['.', '_'])
                        .next()
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(u32::MAX);
                    (2, n)
                } else if let Some(n) = stem
                    .split_once("scale-")
                    .map(|(_, r)| r)
                    .and_then(|r| r.split(['.', '_']).next())
                    .and_then(|v| v.parse::<u32>().ok())
                {
                    if n == 100 {
                        (0, 0)
                    } else {
                        (1, n)
                    }
                } else {
                    (0, 0)
                };
            Some((family, kind, size))
        }

        let mut best: Option<((u8, u8, u32), PathBuf)> = None;
        let mut stack: Vec<(PathBuf, usize)> = vec![(dir.to_path_buf(), 0)];
        while let Some((dir, depth)) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(ft) = entry.file_type() else {
                    continue;
                };
                if ft.is_dir() {
                    if depth < MAX_DEPTH {
                        stack.push((path, depth + 1));
                    }
                    continue;
                }
                if !ft.is_file() {
                    continue;
                }
                let rank_key = entry
                    .file_name()
                    .to_str()
                    .map(|n| n.to_ascii_lowercase())
                    .and_then(|n| rank(&n));
                let Some(rank_key) = rank_key else {
                    continue;
                };
                if best.as_ref().is_none_or(|(r, _)| rank_key < *r) {
                    best = Some((rank_key, path));
                }
            }
        }
        best.map(|(_, p)| p.to_string_lossy().into_owned())
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

        // CR-10: escape single quotes to prevent PowerShell injection
        let safe_name = package_family_name.replace('\'', "''");

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
            safe_name
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

    #[test]
    fn test_find_package_logo_ranking() {
        let root = std::env::temp_dir().join(format!("reek_logo_test_{}", std::process::id()));
        let assets = root.join("Assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("Square44x44Logo.targetsize-16.png"), b"x").unwrap();
        std::fs::write(assets.join("Square150x150Logo.scale-125.png"), b"x").unwrap();
        std::fs::write(root.join("Logo.png"), b"x").unwrap();
        std::fs::write(assets.join("NotALogoCandidate.txt"), b"x").unwrap();

        let got = WindowsStoreScanner::find_package_logo(&root).unwrap();
        assert!(got.ends_with("Square150x150Logo.scale-125.png"));

        std::fs::write(assets.join("Square150x150Logo.scale-100.png"), b"x").unwrap();
        let got = WindowsStoreScanner::find_package_logo(&root).unwrap();
        assert!(got.ends_with("Square150x150Logo.scale-100.png"));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_find_package_logo_missing_dir() {
        assert!(
            WindowsStoreScanner::find_package_logo(Path::new(r"C:\reek_no_such_package_dir"))
                .is_none()
        );
    }
}
