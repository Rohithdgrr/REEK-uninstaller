// Task Scheduler scanner for discovering scheduled tasks

use greek_common::{InstalledApp, Result, GreekError, ArtifactType, LeftoverArtifact, SafetyLevel};
use tracing::{info, warn};
use std::path::PathBuf;
use chrono::NaiveDate;

/// Scheduled task information
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub name: String,
    pub path: String,
    pub state: TaskState,
    pub actions: Vec<TaskAction>,
    pub triggers: Vec<TaskTrigger>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub run_as_user: Option<String>,
    pub last_run_time: Option<String>,
    pub next_run_time: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct TaskAction {
    pub action_type: String,
    pub execute: Option<String>,
    pub arguments: Option<String>,
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskTrigger {
    pub trigger_type: String,
    pub start_boundary: Option<String>,
    pub enabled: bool,
}

/// Task Scheduler scanner
pub struct TaskSchedulerScanner;

impl TaskSchedulerScanner {
    pub fn new() -> Self {
        Self
    }

    /// Scan for all scheduled tasks on Windows
    pub async fn scan_tasks(&self) -> Result<Vec<ScheduledTask>> {
        info!("Scanning Windows Task Scheduler");

        #[cfg(target_os = "windows")]
        {
            self.scan_windows_tasks().await
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(Vec::new())
        }
    }

    #[cfg(target_os = "windows")]
    async fn scan_windows_tasks(&self) -> Result<Vec<ScheduledTask>> {
        use std::process::Command;

        let ps_command = r#"
            Get-ScheduledTask | 
            Select-Object TaskName, TaskPath, State, Author, Description, 
            @{N='Actions';E={$_.Actions | ForEach-Object { 
                @{Type='Execute'; Execute=$_.Execute; Arguments=$_.Arguments; WorkingDirectory=$_.WorkingDirectory}
            }}},
            @{N='Triggers';E={$_.Triggers | ForEach-Object {
                @{Type=$_.CimClass.CimClassName; StartBoundary=$_.StartBoundary; Enabled=$_.Enabled}
            }}},
            @{N='Principal';E={$_.Principal.UserId}},
            @{N='LastRunTime';E={$_.LastRunTime}},
            @{N='NextRunTime';E={$_.NextRunTime}} |
            ConvertTo-Json -Depth 5
        "#;

        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps_command])
            .output()
            .map_err(|e| GreekError::ScanError(format!("Failed to execute PowerShell: {}", e)))?;

        if !output.status.success() {
            warn!("Failed to enumerate scheduled tasks");
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        if json_str.trim().is_empty() || json_str.trim() == "null" {
            return Ok(Vec::new());
        }

        let tasks_json: Vec<serde_json::Value> = serde_json::from_str(&json_str)
            .map_err(|e| GreekError::SystemError(format!("Failed to parse tasks JSON: {}", e)))?;

        let tasks: Vec<ScheduledTask> = tasks_json
            .into_iter()
            .filter_map(|task| self.parse_task_json(&task))
            .collect();

        info!("Found {} scheduled tasks", tasks.len());
        Ok(tasks)
    }

    #[cfg(target_os = "windows")]
    fn parse_task_json(&self, task: &serde_json::Value) -> Option<ScheduledTask> {
        let name = task.get("TaskName")?.as_str()?.to_string();
        let path = task.get("TaskPath")?.as_str()?.to_string();

        let state = match task.get("State")?.as_str()? {
            "Ready" => TaskState::Ready,
            "Running" => TaskState::Running,
            "Disabled" => TaskState::Disabled,
            _ => TaskState::Unknown,
        };

        let actions = task.get("Actions")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|action| {
                        Some(TaskAction {
                            action_type: action.get("Type")?.as_str()?.to_string(),
                            execute: action.get("Execute").and_then(|e| e.as_str()).map(String::from),
                            arguments: action.get("Arguments").and_then(|a| a.as_str()).map(String::from),
                            working_directory: action.get("WorkingDirectory").and_then(|w| w.as_str()).map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let triggers = task.get("Triggers")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|trigger| {
                        Some(TaskTrigger {
                            trigger_type: trigger.get("Type")?.as_str()?.to_string(),
                            start_boundary: trigger.get("StartBoundary").and_then(|s| s.as_str()).map(String::from),
                            enabled: trigger.get("Enabled").and_then(|e| e.as_bool()).unwrap_or(true),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Some(ScheduledTask {
            name,
            path,
            state,
            actions,
            triggers,
            author: task.get("Author").and_then(|a| a.as_str()).map(String::from),
            description: task.get("Description").and_then(|d| d.as_str()).map(String::from),
            run_as_user: task.get("Principal").and_then(|p| p.as_str()).map(String::from),
            last_run_time: task.get("LastRunTime").and_then(|l| l.as_str()).map(String::from),
            next_run_time: task.get("NextRunTime").and_then(|n| n.as_str()).map(String::from),
        })
    }

    /// Find tasks that might be related to an application
    pub async fn find_tasks_for_app(&self, app: &InstalledApp) -> Result<Vec<ScheduledTask>> {
        let all_tasks = self.scan_tasks().await?;
        
        let app_name_lower = app.name.to_lowercase();
        let publisher_lower = app.publisher.as_ref().map(|p| p.to_lowercase());
        
        let matching_tasks: Vec<ScheduledTask> = all_tasks
            .into_iter()
            .filter(|task| {
                let name_match = task.name.to_lowercase().contains(&app_name_lower);
                let action_match = task.actions.iter().any(|action| {
                    action.execute.as_ref()
                        .map(|e| e.to_lowercase().contains(&app_name_lower))
                        .unwrap_or(false)
                });
                let publisher_match = publisher_lower.as_ref()
                    .map(|p| task.name.to_lowercase().contains(p))
                    .unwrap_or(false);
                
                name_match || action_match || publisher_match
            })
            .collect();

        info!("Found {} tasks for app '{}'", matching_tasks.len(), app.name);
        Ok(matching_tasks)
    }

    /// Convert scheduled task to leftover artifact
    pub fn task_to_artifact(&self, task: &ScheduledTask, app_id: Option<uuid::Uuid>) -> LeftoverArtifact {
        let mut artifact = LeftoverArtifact::new(
            ArtifactType::ScheduledTask,
            PathBuf::from(&task.path),
        );
        artifact.app_id = app_id;
        artifact.description = format!("Scheduled task: {}", task.name);
        artifact.safety_level = SafetyLevel::Caution;
        artifact.confidence = 0.7;
        artifact
    }

    /// Delete a scheduled task
    pub async fn delete_task(&self, task_path: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            info!("Deleting scheduled task: {}", task_path);

            let ps_command = format!(
                r#"
                Unregister-ScheduledTask -TaskPath '{}' -Confirm:$false -ErrorAction Stop
                "#,
                task_path.replace('\'', "''")
            );

            let output = Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps_command])
                .output()
                .map_err(|e| GreekError::SystemError(format!("Failed to delete task: {}", e)))?;

            if output.status.success() {
                info!("Successfully deleted task: {}", task_path);
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(GreekError::SystemError(format!("Failed to delete task: {}", stderr)))
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err(GreekError::SystemError("Task Scheduler not available on this platform".to_string()))
        }
    }
}

impl Default for TaskSchedulerScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_scanner_creation() {
        let scanner = TaskSchedulerScanner::new();
        assert!(true);
    }

    #[tokio::test]
    async fn test_scan_tasks() {
        let scanner = TaskSchedulerScanner::new();
        let result = scanner.scan_tasks().await;
        assert!(result.is_ok());
    }
}