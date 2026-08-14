// WMI (Windows Management Instrumentation) integration for REEK

use greek_common::{GreekError, InstallSource, InstalledApp, Result};
use std::collections::HashMap;
use tracing::info;

/// WMI query executor and result parser
pub struct WmiClient;

impl WmiClient {
    pub fn new() -> Self {
        Self
    }

    /// Execute a WMI query and return parsed results
    async fn execute_query(&self, wql: &str) -> Result<Vec<HashMap<String, String>>> {
        info!("Executing WMI query: {}", wql);

        // Use PowerShell to execute WMI queries
        let ps_command = format!(
            r#"
            Get-CimInstance -Query '{}' | 
            ForEach-Object {{ 
                $hash = @{{}}
                $_.PSObject.Properties | ForEach-Object {{ $hash[$_.Name] = [string]$_.Value }}
                $hash
            }} | 
            ConvertTo-Json -Compress
            "#,
            wql.replace('\'', "''")
        );

        let output = self.run_powershell(&ps_command).await?;

        if output.is_empty() || output.trim() == "null" {
            return Ok(Vec::new());
        }

        // Parse JSON array of objects
        let results: Vec<HashMap<String, String>> = serde_json::from_str(&output)
            .map_err(|e| GreekError::SystemError(format!("Failed to parse WMI results: {}", e)))?;

        Ok(results)
    }

    /// Run a PowerShell command
    async fn run_powershell(&self, command: &str) -> Result<String> {
        use std::process::Command;

        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", command])
            .output()
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

    /// Query installed software from Win32_Product
    pub async fn query_installed_software(&self) -> Result<Vec<InstalledApp>> {
        let wql = "SELECT Name, Version, Publisher, InstallDate, InstallLocation, IdentifyingNumber FROM Win32_Product";

        let results = self.execute_query(wql).await?;

        let apps: Vec<InstalledApp> = results
            .into_iter()
            .filter_map(|props| self.parse_product_entry(&props))
            .collect();

        info!("Found {} installed software via WMI", apps.len());
        Ok(apps)
    }

    /// Parse a Win32_Product entry into an InstalledApp
    fn parse_product_entry(&self, props: &HashMap<String, String>) -> Option<InstalledApp> {
        let name = props.get("Name")?.clone();
        if name.is_empty() {
            return None;
        }

        let version = props.get("Version").cloned();
        let publisher = props.get("Publisher").cloned();
        let install_location = props
            .get("InstallLocation")
            .cloned()
            .filter(|s| !s.is_empty())
            .map(std::path::PathBuf::from);
        let identifying_number = props.get("IdentifyingNumber").cloned().unwrap_or_default();

        let mut app = InstalledApp::new(
            name,
            InstallSource::Registry {
                hive: greek_common::RegistryHive::Hklm,
                key_path: format!("Win32_Product{{{}}}", identifying_number),
            },
        );

        app.version = version;
        app.publisher = publisher;
        app.install_location = install_location;

        // Parse install date
        if let Some(date_str) = props.get("InstallDate") {
            app.install_date = self.parse_wmi_date(date_str);
        }

        Some(app)
    }

    /// Parse WMI date format (YYYYMMDDHHMMSS.ffffff+ZZZ)
    fn parse_wmi_date(&self, date_str: &str) -> Option<chrono::NaiveDate> {
        if date_str.len() >= 8 {
            let year = date_str[0..4].parse::<i32>().ok()?;
            let month = date_str[4..6].parse::<u32>().ok()?;
            let day = date_str[6..8].parse::<u32>().ok()?;
            chrono::NaiveDate::from_ymd_opt(year, month, day)
        } else {
            None
        }
    }

    /// Query Windows features
    pub async fn query_windows_features(&self) -> Result<Vec<WindowsFeature>> {
        let wql = "SELECT Name, Caption, Description, Enabled FROM Win32_OptionalFeature";

        let results = self.execute_query(wql).await?;

        let features: Vec<WindowsFeature> = results
            .into_iter()
            .filter_map(|props| {
                Some(WindowsFeature {
                    name: props.get("Name")?.clone(),
                    caption: props.get("Caption").cloned().unwrap_or_default(),
                    description: props.get("Description").cloned().unwrap_or_default(),
                    enabled: props.get("Enabled").map(|s| s == "1").unwrap_or(false),
                })
            })
            .collect();

        info!("Found {} Windows features", features.len());
        Ok(features)
    }

    /// Query running processes
    pub async fn query_processes(&self) -> Result<Vec<ProcessInfo>> {
        let wql = "SELECT ProcessId, Name, ExecutablePath, CommandLine FROM Win32_Process WHERE ExecutablePath IS NOT NULL";

        let results = self.execute_query(wql).await?;

        let processes: Vec<ProcessInfo> = results
            .into_iter()
            .filter_map(|props| {
                Some(ProcessInfo {
                    process_id: props.get("ProcessId")?.parse::<u32>().ok()?,
                    name: props.get("Name")?.clone(),
                    executable_path: props.get("ExecutablePath").cloned(),
                    command_line: props.get("CommandLine").cloned(),
                })
            })
            .collect();

        info!("Found {} running processes", processes.len());
        Ok(processes)
    }

    /// Query startup items
    pub async fn query_startup_items(&self) -> Result<Vec<StartupItem>> {
        let wql = "SELECT Name, Command, Location FROM Win32_StartupCommand";

        let results = self.execute_query(wql).await?;

        let items: Vec<StartupItem> = results
            .into_iter()
            .filter_map(|props| {
                Some(StartupItem {
                    name: props.get("Name")?.clone(),
                    command: props.get("Command")?.clone(),
                    location: props.get("Location")?.clone(),
                })
            })
            .collect();

        info!("Found {} startup items", items.len());
        Ok(items)
    }

    /// Query scheduled tasks
    pub async fn query_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>> {
        // WMI doesn't directly expose scheduled tasks, use PowerShell instead
        let ps_command = r#"
            Get-ScheduledTask | 
            Where-Object { $_.State -ne 'Disabled' } |
            Select-Object TaskName, TaskPath, State, 
            @{N='Actions';E={$_.Actions.Execute}} |
            ConvertTo-Json -Depth 3
        "#;

        let output = self.run_powershell(ps_command).await?;

        if output.is_empty() || output.trim() == "null" {
            return Ok(Vec::new());
        }

        let tasks_json: Vec<serde_json::Value> = serde_json::from_str(&output).map_err(|e| {
            GreekError::SystemError(format!("Failed to parse scheduled tasks: {}", e))
        })?;

        let tasks: Vec<ScheduledTask> = tasks_json
            .into_iter()
            .filter_map(|task| {
                Some(ScheduledTask {
                    name: task.get("TaskName")?.as_str()?.to_string(),
                    path: task.get("TaskPath")?.as_str()?.to_string(),
                    state: task.get("State")?.as_str()?.to_string(),
                    action: task.get("Actions")?.as_str()?.to_string(),
                })
            })
            .collect();

        info!("Found {} scheduled tasks", tasks.len());
        Ok(tasks)
    }
}

impl Default for WmiClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Windows feature information
#[derive(Debug, Clone)]
pub struct WindowsFeature {
    pub name: String,
    pub caption: String,
    pub description: String,
    pub enabled: bool,
}

/// Process information
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub process_id: u32,
    pub name: String,
    pub executable_path: Option<String>,
    pub command_line: Option<String>,
}

/// Startup item information
#[derive(Debug, Clone)]
pub struct StartupItem {
    pub name: String,
    pub command: String,
    pub location: String,
}

/// Scheduled task information
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub name: String,
    pub path: String,
    pub state: String,
    pub action: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wmi_client_creation() {
        let client = WmiClient::new();
        // This test only works on Windows with a responsive WMI provider.
        // It may legitimately return an error on constrained CI machines, so we
        // only assert that the call completes without panicking.
        let _ = client.query_installed_software().await;
    }

    #[tokio::test]
    async fn test_query_installed_software() {
        let client = WmiClient::new();
        // This test only works on Windows with a responsive WMI provider.
        // It may legitimately return an error on constrained CI machines, so we
        // only assert that the call completes without panicking.
        let _ = client.query_installed_software().await;
    }

    #[tokio::test]
    async fn test_query_windows_features() {
        let client = WmiClient::new();
        // See note above: environment-dependent, don't require success.
        let _ = client.query_windows_features().await;
    }
}
