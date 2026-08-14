// Linux-specific implementations for package management

use greek_common::{InstalledApp, InstallSource, Result, GreekError, PackageManager};
use tracing::info;
use std::process::Command;

/// Linux package manager scanner
pub struct LinuxPackageScanner {
    package_manager: LinuxPackageManager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxPackageManager {
    Apt,
    Dpkg,
    Rpm,
    Pacman,
    Flatpak,
    Snap,
}

impl LinuxPackageScanner {
    pub fn new(package_manager: LinuxPackageManager) -> Self {
        Self { package_manager }
    }

    /// Detect the system's package manager
    pub fn detect_package_manager() -> Option<LinuxPackageManager> {
        if Command::new("apt").arg("--version").output().is_ok() {
            return Some(LinuxPackageManager::Apt);
        }
        if Command::new("dpkg").arg("--version").output().is_ok() {
            return Some(LinuxPackageManager::Dpkg);
        }
        if Command::new("rpm").arg("--version").output().is_ok() {
            return Some(LinuxPackageManager::Rpm);
        }
        if Command::new("pacman").arg("--version").output().is_ok() {
            return Some(LinuxPackageManager::Pacman);
        }
        if Command::new("flatpak").arg("--version").output().is_ok() {
            return Some(LinuxPackageManager::Flatpak);
        }
        if Command::new("snap").arg("--version").output().is_ok() {
            return Some(LinuxPackageManager::Snap);
        }
        None
    }

    /// Scan for installed packages
    pub async fn scan(&self) -> Result<Vec<InstalledApp>> {
        info!("Starting Linux package scan with {:?}", self.package_manager);
        
        let apps = match self.package_manager {
            LinuxPackageManager::Apt => self.scan_apt().await?,
            LinuxPackageManager::Dpkg => self.scan_dpkg().await?,
            LinuxPackageManager::Rpm => self.scan_rpm().await?,
            LinuxPackageManager::Pacman => self.scan_pacman().await?,
            LinuxPackageManager::Flatpak => self.scan_flatpak().await?,
            LinuxPackageManager::Snap => self.scan_snap().await?,
        };

        info!("Found {} packages via {:?}", apps.len(), self.package_manager);
        Ok(apps)
    }

    /// Scan using apt/dpkg (Debian/Ubuntu)
    async fn scan_apt(&self) -> Result<Vec<InstalledApp>> {
        let output = Command::new("dpkg-query")
            .args(["-W", "-f", "${Package}\t${Version}\t${Status}\t${Description}\n"])
            .output()
            .map_err(|e| GreekError::ScanError(format!("Failed to execute dpkg-query: {}", e)))?;

        if !output.status.success() {
            return Err(GreekError::ScanError("dpkg-query failed".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut apps = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();
                let status = parts[2];
                let description = parts[3];

                // Only include installed packages
                if status.contains("install ok installed") {
                    let mut app = InstalledApp::new(
                        name.clone(),
                        InstallSource::PackageManager {
                            manager: PackageManager::Dpkg,
                            package_id: name,
                        },
                    );
                    app.version = Some(version);
                    app.publisher = Some(description.to_string());
                    apps.push(app);
                }
            }
        }

        Ok(apps)
    }

    /// Scan using dpkg directly
    async fn scan_dpkg(&self) -> Result<Vec<InstalledApp>> {
        let output = Command::new("dpkg")
            .args(["-l"])
            .output()
            .map_err(|e| GreekError::ScanError(format!("Failed to execute dpkg: {}", e)))?;

        if !output.status.success() {
            return Err(GreekError::ScanError("dpkg failed".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut apps = Vec::new();

        for line in stdout.lines().skip(5) { // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == "ii" {
                let name = parts[1].to_string();
                let version = parts[2].to_string();

                let mut app = InstalledApp::new(
                    name.clone(),
                    InstallSource::PackageManager {
                        manager: PackageManager::Dpkg,
                        package_id: name,
                    },
                );
                app.version = Some(version);
                apps.push(app);
            }
        }

        Ok(apps)
    }

    /// Scan using rpm (Fedora/RHEL/CentOS)
    async fn scan_rpm(&self) -> Result<Vec<InstalledApp>> {
        let output = Command::new("rpm")
            .args(["-qa", "--queryformat", "%{NAME}\t%{VERSION}-%{RELEASE}\t%{VENDOR}\t%{SUMMARY}\n"])
            .output()
            .map_err(|e| GreekError::ScanError(format!("Failed to execute rpm: {}", e)))?;

        if !output.status.success() {
            return Err(GreekError::ScanError("rpm failed".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut apps = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();
                let vendor = parts[2];
                let summary = parts[3];

                let mut app = InstalledApp::new(
                    name.clone(),
                    InstallSource::PackageManager {
                        manager: PackageManager::Rpm,
                        package_id: name,
                    },
                );
                app.version = Some(version);
                app.publisher = Some(vendor.to_string());
                app.metadata.insert("description".to_string(), summary.to_string());
                apps.push(app);
            }
        }

        Ok(apps)
    }

    /// Scan using pacman (Arch Linux)
    async fn scan_pacman(&self) -> Result<Vec<InstalledApp>> {
        let output = Command::new("pacman")
            .args(["-Q"])
            .output()
            .map_err(|e| GreekError::ScanError(format!("Failed to execute pacman: {}", e)))?;

        if !output.status.success() {
            return Err(GreekError::ScanError("pacman failed".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut apps = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();

                let mut app = InstalledApp::new(
                    name.clone(),
                    InstallSource::PackageManager {
                        manager: PackageManager::Pacman,
                        package_id: name,
                    },
                );
                app.version = Some(version);
                apps.push(app);
            }
        }

        Ok(apps)
    }

    /// Scan using Flatpak
    async fn scan_flatpak(&self) -> Result<Vec<InstalledApp>> {
        let output = Command::new("flatpak")
            .args(["list", "--app", "--columns=application,version,origin,description"])
            .output()
            .map_err(|e| GreekError::ScanError(format!("Failed to execute flatpak: {}", e)))?;

        if !output.status.success() {
            return Err(GreekError::ScanError("flatpak failed".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut apps = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();
                let origin = parts[2];
                let description = parts[3];

                let mut app = InstalledApp::new(
                    name.clone(),
                    InstallSource::PackageManager {
                        manager: PackageManager::Flatpak,
                        package_id: name,
                    },
                );
                app.version = Some(version);
                app.publisher = Some(origin.to_string());
                app.metadata.insert("description".to_string(), description.to_string());
                apps.push(app);
            }
        }

        Ok(apps)
    }

    /// Scan using Snap
    async fn scan_snap(&self) -> Result<Vec<InstalledApp>> {
        let output = Command::new("snap")
            .args(["list"])
            .output()
            .map_err(|e| GreekError::ScanError(format!("Failed to execute snap: {}", e)))?;

        if !output.status.success() {
            return Err(GreekError::ScanError("snap failed".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut apps = Vec::new();

        for line in stdout.lines().skip(1) { // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();
                let rev = parts[2];
                let tracking = parts[3];

                let mut app = InstalledApp::new(
                    name.clone(),
                    InstallSource::PackageManager {
                        manager: PackageManager::Snap,
                        package_id: name,
                    },
                );
                app.version = Some(version);
                app.metadata.insert("revision".to_string(), rev.to_string());
                app.metadata.insert("tracking".to_string(), tracking.to_string());
                apps.push(app);
            }
        }

        Ok(apps)
    }

    /// Remove a package
    pub async fn remove_package(&self, package_name: &str) -> Result<()> {
        info!("Removing package: {} via {:?}", package_name, self.package_manager);

        let result = match self.package_manager {
            LinuxPackageManager::Apt | LinuxPackageManager::Dpkg => {
                Command::new("sudo")
                    .args(["apt", "remove", "-y", package_name])
                    .output()
            }
            LinuxPackageManager::Rpm => {
                Command::new("sudo")
                    .args(["dnf", "remove", "-y", package_name])
                    .output()
            }
            LinuxPackageManager::Pacman => {
                Command::new("sudo")
                    .args(["pacman", "-R", "--noconfirm", package_name])
                    .output()
            }
            LinuxPackageManager::Flatpak => {
                Command::new("flatpak")
                    .args(["uninstall", "-y", package_name])
                    .output()
            }
            LinuxPackageManager::Snap => {
                Command::new("sudo")
                    .args(["snap", "remove", package_name])
                    .output()
            }
        };

        match result {
            Ok(output) => {
                if output.status.success() {
                    info!("Successfully removed package: {}", package_name);
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(GreekError::UninstallError(format!("Failed to remove package: {}", stderr)))
                }
            }
            Err(e) => Err(GreekError::UninstallError(format!("Failed to execute remove command: {}", e))),
        }
    }

    /// Get package information
    pub async fn get_package_info(&self, package_name: &str) -> Result<PackageInfo> {
        let output = match self.package_manager {
            LinuxPackageManager::Apt | LinuxPackageManager::Dpkg => {
                Command::new("dpkg-query")
                    .args(["-W", "-f", "${Package}\n${Version}\n${Status}\n${Description}\n", package_name])
                    .output()
            }
            LinuxPackageManager::Rpm => {
                Command::new("rpm")
                    .args(["-qi", package_name])
                    .output()
            }
            LinuxPackageManager::Pacman => {
                Command::new("pacman")
                    .args(["-Qi", package_name])
                    .output()
            }
            _ => {
                return Err(GreekError::SystemError("Package info not supported for this manager".to_string()));
            }
        };

        let output = output.map_err(|e| GreekError::SystemError(format!("Failed to get package info: {}", e)))?;

        if !output.status.success() {
            return Err(GreekError::NotFound(format!("Package '{}' not found", package_name)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse output based on package manager
        Ok(PackageInfo {
            name: package_name.to_string(),
            version: "unknown".to_string(),
            description: stdout.lines().next().unwrap_or("").to_string(),
            size: None,
        })
    }
}

impl Default for LinuxPackageScanner {
    fn default() -> Self {
        Self::new(LinuxPackageManager::Apt)
    }
}

/// Package information
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub size: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_package_manager() {
        let pm = LinuxPackageScanner::detect_package_manager();
        // On Linux systems, this should return Some
        // On other systems, it returns None
        println!("Detected package manager: {:?}", pm);
    }

    #[tokio::test]
    async fn test_scanner_creation() {
        let scanner = LinuxPackageScanner::new(LinuxPackageManager::Apt);
        assert_eq!(scanner.package_manager, LinuxPackageManager::Apt);
    }
}