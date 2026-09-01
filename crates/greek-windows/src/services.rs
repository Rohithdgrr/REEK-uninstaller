// Windows service management implementation

use greek_common::{GreekError, InstalledApp, Result};
use tracing::{info, warn};

/// Windows service information
#[derive(Debug, Clone)]
pub struct WindowsService {
    pub name: String,
    pub display_name: String,
    pub status: ServiceStatus,
    pub process_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Paused,
    StartPending,
    StopPending,
    Unknown,
}

/// Windows service manager implementation using PowerShell
pub struct WindowsServiceManager;

impl WindowsServiceManager {
    pub fn new() -> Self {
        Self
    }

    /// Find services associated with an application
    pub async fn find_services_for_app(&self, app: &InstalledApp) -> Result<Vec<WindowsService>> {
        info!("Finding services for app: {}", app.name);

        let app_name_lower = app.name.to_lowercase();
        let install_lower = app
            .install_location
            .as_ref()
            .map(|p| p.to_string_lossy().to_lowercase());
        let all_services = self.list_all_services().await?;

        let matching: Vec<WindowsService> = all_services
            .into_iter()
            .filter(|svc| {
                // Name / display name match
                if svc.name.to_lowercase().contains(&app_name_lower)
                    || svc.display_name.to_lowercase().contains(&app_name_lower)
                {
                    return true;
                }
                // For future: if we had PathName per service, match install_location prefix.
                // Currently list_all_services does not expose PathName; name match is conservative.
                // If install_location present, consider it for filtering via secondary WMI query.
                let _ = &install_lower;
                false
            })
            .collect();

        info!("Found {} services for app '{}'", matching.len(), app.name);
        Ok(matching)
    }

    /// List all Windows services — uses Win32_Service to get real ProcessId.
    pub async fn list_all_services(&self) -> Result<Vec<WindowsService>> {
        use std::process::Command;

        // Use CimInstance which exposes ProcessId; Get-Service does not.
        let ps_command = r#"
            Get-CimInstance Win32_Service | Select-Object Name, DisplayName, State, ProcessId | ConvertTo-Json
        "#;

        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps_command])
            .output()
            .map_err(|e| GreekError::SystemError(format!("Failed to execute PowerShell: {}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let trimmed = json_str.trim();
        if trimmed.is_empty() || trimmed == "null" {
            return Ok(Vec::new());
        }

        // PowerShell emits single object when one service; handle both array & object
        let values: Vec<serde_json::Value> = if trimmed.starts_with('[') {
            serde_json::from_str(trimmed)
                .map_err(|e| GreekError::SystemError(format!("Failed to parse services: {}", e)))?
        } else {
            let v: serde_json::Value = serde_json::from_str(trimmed)
                .map_err(|e| GreekError::SystemError(format!("Failed to parse services: {}", e)))?;
            vec![v]
        };

        let services: Vec<WindowsService> = values
            .into_iter()
            .filter_map(|svc| {
                let name = svc.get("Name")?.as_str()?.to_string();
                let display_name = svc.get("DisplayName")?.as_str()?.to_string();
                // Win32_Service uses "State", fallback to "Status" for compat
                let status_str = svc
                    .get("State")
                    .or_else(|| svc.get("Status"))
                    .and_then(|v| v.as_str())?;
                let status = match status_str {
                    "Running" => ServiceStatus::Running,
                    "Stopped" => ServiceStatus::Stopped,
                    "Paused" => ServiceStatus::Paused,
                    "Start Pending" | "StartPending" => ServiceStatus::StartPending,
                    "Stop Pending" | "StopPending" => ServiceStatus::StopPending,
                    _ => ServiceStatus::Unknown,
                };
                let pid = svc.get("ProcessId").and_then(|v| v.as_u64()).and_then(|n| {
                    if n == 0 {
                        None
                    } else {
                        Some(n as u32)
                    }
                });
                Some(WindowsService {
                    name,
                    display_name,
                    status,
                    process_id: pid,
                })
            })
            .collect();

        Ok(services)
    }

    /// Stop a service
    pub async fn stop_service(&self, service_name: &str) -> Result<()> {
        info!("Stopping service: {}", service_name);

        use std::process::Command;

        let ps_command = format!(
            r#"Stop-Service -Name '{}' -Force -ErrorAction Stop"#,
            service_name.replace('\'', "''")
        );

        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_command])
            .output()
            .map_err(|e| GreekError::SystemError(format!("Failed to stop service: {}", e)))?;

        if output.status.success() {
            info!("Service '{}' stopped successfully", service_name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GreekError::ServiceError(format!(
                "Failed to stop service: {}",
                stderr
            )))
        }
    }

    /// Delete a service
    pub async fn delete_service(&self, service_name: &str) -> Result<()> {
        info!("Deleting service: {}", service_name);

        use std::process::Command;

        let ps_command = format!(
            r#"Get-Service -Name '{}' | Stop-Service -Force; sc.exe delete '{}'"#,
            service_name.replace('\'', "''"),
            service_name.replace('\'', "''")
        );

        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_command])
            .output()
            .map_err(|e| GreekError::SystemError(format!("Failed to delete service: {}", e)))?;

        if output.status.success() {
            info!("Service '{}' deleted successfully", service_name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GreekError::ServiceError(format!(
                "Failed to delete service: {}",
                stderr
            )))
        }
    }

    /// Clean up all services associated with an application
    pub async fn cleanup_app_services(&self, app: &InstalledApp) -> Result<Vec<String>> {
        let services = self.find_services_for_app(app).await?;
        let mut cleaned = Vec::new();

        for service in &services {
            if service.status == ServiceStatus::Running {
                if let Err(e) = self.stop_service(&service.name).await {
                    warn!("Failed to stop service {}: {}", service.name, e);
                    continue;
                }
            }

            if let Err(e) = self.delete_service(&service.name).await {
                warn!("Failed to delete service {}: {}", service.name, e);
            } else {
                cleaned.push(service.name.clone());
                info!("Cleaned up service: {}", service.name);
            }
        }

        Ok(cleaned)
    }
}

impl Default for WindowsServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_manager_creation() {
        let manager = WindowsServiceManager::new();
        // Environment-dependent, so only verify the manager is usable.
        let _ = manager.list_all_services().await;
    }
}
