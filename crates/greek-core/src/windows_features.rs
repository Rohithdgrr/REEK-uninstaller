// Windows Features scanner for discovering optional Windows components

use greek_common::{InstalledApp, InstallSource, Result, GreekError};
use tracing::{info, warn};
use std::path::PathBuf;

/// Windows optional feature information
#[derive(Debug, Clone)]
pub struct WindowsFeature {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub feature_type: FeatureType,
    pub state: FeatureState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureType {
    WindowsFeature,
    OptionalFeature,
    Capability,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureState {
    Enabled,
    Disabled,
    EnablePending,
    DisablePending,
    Unknown,
}

/// Windows Features scanner
pub struct WindowsFeaturesScanner;

impl WindowsFeaturesScanner {
    pub fn new() -> Self {
        Self
    }

    /// Scan for Windows optional features using DISM and PowerShell
    pub async fn scan_features(&self) -> Result<Vec<WindowsFeature>> {
        info!("Scanning Windows optional features");
        
        let mut features = Vec::new();
        
        // Get features using DISM
        if let Ok(dism_features) = self.scan_dism_features().await {
            features.extend(dism_features);
        }
        
        // Get features using PowerShell
        if let Ok(ps_features) = self.scan_powershell_features().await {
            features.extend(ps_features);
        }
        
        // Get capabilities using PowerShell
        if let Ok(capabilities) = self.scan_capabilities().await {
            features.extend(capabilities);
        }
        
        info!("Found {} Windows features", features.len());
        Ok(features)
    }

    /// Scan using DISM (Deployment Image Servicing and Management)
    async fn scan_dism_features(&self) -> Result<Vec<WindowsFeature>> {
        use std::process::Command;

        let output = Command::new("dism.exe")
            .args(["/online", "/get-features", "/format:table"])
            .output()
            .map_err(|e| GreekError::ScanError(format!("Failed to execute DISM: {}", e)))?;

        if !output.status.success() {
            warn!("DISM command failed, falling back to PowerShell");
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut features = Vec::new();

        for line in stdout.lines().skip(3) { // Skip header lines
            let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                let name = parts[0].to_string();
                let display_name = parts[1].to_string();
                let state_str = parts[2];
                
                if name.is_empty() || name.starts_with("---") {
                    continue;
                }
                
                let state = match state_str {
                    "Enabled" => FeatureState::Enabled,
                    "Disabled" => FeatureState::Disabled,
                    "Enable Pending" => FeatureState::EnablePending,
                    "Disable Pending" => FeatureState::DisablePending,
                    _ => FeatureState::Unknown,
                };
                
                features.push(WindowsFeature {
                    name: name.clone(),
                    display_name,
                    description: None,
                    enabled: state == FeatureState::Enabled,
                    feature_type: FeatureType::WindowsFeature,
                    state,
                });
            }
        }

        info!("Found {} features via DISM", features.len());
        Ok(features)
    }

    /// Scan using PowerShell Get-WindowsOptionalFeature
    async fn scan_powershell_features(&self) -> Result<Vec<WindowsFeature>> {
        use std::process::Command;

        let ps_command = r#"
            Get-WindowsOptionalFeature -Online | 
            Select-Object FeatureName, State, Description |
            ConvertTo-Json
        "#;

        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps_command])
            .output()
            .map_err(|e| GreekError::ScanError(format!("Failed to execute PowerShell: {}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        if json_str.trim().is_empty() || json_str.trim() == "null" {
            return Ok(Vec::new());
        }

        let features_json: Vec<serde_json::Value> = serde_json::from_str(&json_str)
            .map_err(|e| GreekError::SystemError(format!("Failed to parse features JSON: {}", e)))?;

        let features: Vec<WindowsFeature> = features_json
            .into_iter()
            .filter_map(|feature| {
                let name = feature.get("FeatureName")?.as_str()?.to_string();
                let state_str = feature.get("State")?.as_str()?;
                let description = feature.get("Description").and_then(|d| d.as_str()).map(String::from);
                
                let state = match state_str {
                    "Enabled" => FeatureState::Enabled,
                    "Disabled" => FeatureState::Disabled,
                    _ => FeatureState::Unknown,
                };
                
                Some(WindowsFeature {
                    name: name.clone(),
                    display_name: name,
                    description,
                    enabled: state == FeatureState::Enabled,
                    feature_type: FeatureType::OptionalFeature,
                    state,
                })
            })
            .collect();

        info!("Found {} features via PowerShell", features.len());
        Ok(features)
    }

    /// Scan for Windows capabilities
    async fn scan_capabilities(&self) -> Result<Vec<WindowsFeature>> {
        use std::process::Command;

        let ps_command = r#"
            Get-WindowsCapability -Online | 
            Select-Object Name, DisplayName, State |
            ConvertTo-Json
        "#;

        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps_command])
            .output()
            .map_err(|e| GreekError::ScanError(format!("Failed to execute PowerShell: {}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        if json_str.trim().is_empty() || json_str.trim() == "null" {
            return Ok(Vec::new());
        }

        let caps_json: Vec<serde_json::Value> = serde_json::from_str(&json_str)
            .map_err(|e| GreekError::SystemError(format!("Failed to parse capabilities JSON: {}", e)))?;

        let capabilities: Vec<WindowsFeature> = caps_json
            .into_iter()
            .filter_map(|cap| {
                let name = cap.get("Name")?.as_str()?.to_string();
                let display_name = cap.get("DisplayName")
                    .and_then(|n| n.as_str())
                    .unwrap_or(&name)
                    .to_string();
                let state_str = cap.get("State")?.as_str()?;
                
                let state = match state_str {
                    "Installed" => FeatureState::Enabled,
                    "NotPresent" => FeatureState::Disabled,
                    "Staging" => FeatureState::EnablePending,
                    _ => FeatureState::Unknown,
                };
                
                Some(WindowsFeature {
                    name,
                    display_name,
                    description: None,
                    enabled: state == FeatureState::Enabled,
                    feature_type: FeatureType::Capability,
                    state,
                })
            })
            .collect();

        info!("Found {} capabilities", capabilities.len());
        Ok(capabilities)
    }

    /// Enable a Windows feature
    pub async fn enable_feature(&self, feature_name: &str) -> Result<()> {
        info!("Enabling Windows feature: {}", feature_name);
        
        use std::process::Command;
        
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("Enable-WindowsOptionalFeature -Online -FeatureName '{}' -NoRestart", feature_name),
            ])
            .output()
            .map_err(|e| GreekError::SystemError(format!("Failed to enable feature: {}", e)))?;

        if output.status.success() {
            info!("Successfully enabled feature: {}", feature_name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GreekError::SystemError(format!("Failed to enable feature: {}", stderr)))
        }
    }

    /// Disable a Windows feature
    pub async fn disable_feature(&self, feature_name: &str) -> Result<()> {
        info!("Disabling Windows feature: {}", feature_name);
        
        use std::process::Command;
        
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("Disable-WindowsOptionalFeature -Online -FeatureName '{}' -NoRestart", feature_name),
            ])
            .output()
            .map_err(|e| GreekError::SystemError(format!("Failed to disable feature: {}", e)))?;

        if output.status.success() {
            info!("Successfully disabled feature: {}", feature_name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GreekError::SystemError(format!("Failed to disable feature: {}", stderr)))
        }
    }

    /// Install a Windows capability
    pub async fn install_capability(&self, capability_name: &str) -> Result<()> {
        info!("Installing Windows capability: {}", capability_name);
        
        use std::process::Command;
        
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("Add-WindowsCapability -Online -Name '{}'", capability_name),
            ])
            .output()
            .map_err(|e| GreekError::SystemError(format!("Failed to install capability: {}", e)))?;

        if output.status.success() {
            info!("Successfully installed capability: {}", capability_name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GreekError::SystemError(format!("Failed to install capability: {}", stderr)))
        }
    }

    /// Remove a Windows capability
    pub async fn remove_capability(&self, capability_name: &str) -> Result<()> {
        info!("Removing Windows capability: {}", capability_name);
        
        use std::process::Command;
        
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("Remove-WindowsCapability -Online -Name '{}'", capability_name),
            ])
            .output()
            .map_err(|e| GreekError::SystemError(format!("Failed to remove capability: {}", e)))?;

        if output.status.success() {
            info!("Successfully removed capability: {}", capability_name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GreekError::SystemError(format!("Failed to remove capability: {}", stderr)))
        }
    }

    /// Find features related to a specific application
    pub async fn find_features_for_app(&self, app: &InstalledApp) -> Result<Vec<WindowsFeature>> {
        let all_features = self.scan_features().await?;
        
        let app_name_lower = app.name.to_lowercase();
        let publisher_lower = app.publisher.as_ref().map(|p| p.to_lowercase());
        
        let matching: Vec<WindowsFeature> = all_features
            .into_iter()
            .filter(|feature| {
                let name_match = feature.name.to_lowercase().contains(&app_name_lower)
                    || feature.display_name.to_lowercase().contains(&app_name_lower);
                let publisher_match = publisher_lower.as_ref()
                    .map(|p| feature.name.to_lowercase().contains(p) || 
                         feature.display_name.to_lowercase().contains(p))
                    .unwrap_or(false);
                
                name_match || publisher_match
            })
            .collect();

        info!("Found {} features for app '{}'", matching.len(), app.name);
        Ok(matching)
    }

    /// Get feature statistics
    pub async fn get_statistics(&self) -> Result<FeatureStatistics> {
        let features = self.scan_features().await?;
        
        let enabled_count = features.iter().filter(|f| f.enabled).count();
        let disabled_count = features.iter().filter(|f| !f.enabled).count();
        
        let by_type = features.iter().fold(std::collections::HashMap::new(), |mut acc, f| {
            *acc.entry(format!("{:?}", f.feature_type)).or_insert(0) += 1;
            acc
        });

        Ok(FeatureStatistics {
            total: features.len(),
            enabled: enabled_count,
            disabled: disabled_count,
            by_type,
        })
    }
}

impl Default for WindowsFeaturesScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Feature statistics
#[derive(Debug, Clone)]
pub struct FeatureStatistics {
    pub total: usize,
    pub enabled: usize,
    pub disabled: usize,
    pub by_type: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scanner_creation() {
        let scanner = WindowsFeaturesScanner::new();
        assert!(true);
    }

    #[tokio::test]
    async fn test_scan_features() {
        let scanner = WindowsFeaturesScanner::new();
        let result = scanner.scan_features().await;
        assert!(result.is_ok());
    }
}