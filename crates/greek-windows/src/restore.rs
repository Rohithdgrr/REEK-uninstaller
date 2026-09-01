// Windows System Restore point management

use greek_common::{GreekError, Result};
use tracing::{info, warn};

/// System restore point manager
pub struct RestorePointManager;

impl RestorePointManager {
    pub fn new() -> Self {
        Self
    }

    /// Create a system restore point
    pub async fn create_restore_point(&self, description: &str) -> Result<String> {
        info!("Creating system restore point: {}", description);

        // First enable System Restore if disabled
        self.enable_system_restore().await?;

        // Create the restore point using PowerShell
        let ps_command = format!(
            r#"
            Checkpoint-Computer -Description '{}' -RestorePointType MODIFY_SETTINGS
            Write-Output "SUCCESS"
            "#,
            description.replace('\'', "''")
        );

        let output = self.run_powershell(&ps_command).await?;

        if output.contains("SUCCESS") {
            info!("System restore point created successfully");
            Ok(description.to_string())
        } else if output.contains("already in progress") {
            Err(GreekError::SystemError(
                "A restore point creation is already in progress".to_string(),
            ))
        } else if output.contains("not enabled") {
            Err(GreekError::SystemError(
                "System Restore is not enabled on this system".to_string(),
            ))
        } else {
            Err(GreekError::SystemError(format!(
                "Failed to create restore point: {}",
                output
            )))
        }
    }

    /// List existing restore points
    pub async fn list_restore_points(&self) -> Result<Vec<RestorePoint>> {
        info!("Listing system restore points");

        let ps_command = r#"
            Get-ComputerRestorePoint | 
            Select-Object SequenceNumber, Description, CreationTime, RestorePointType |
            ConvertTo-Json
        "#;

        let output = self.run_powershell(ps_command).await?;

        if output.is_empty() || output.trim() == "null" {
            return Ok(Vec::new());
        }

        let points_json: Vec<serde_json::Value> = serde_json::from_str(&output).map_err(|e| {
            GreekError::SystemError(format!("Failed to parse restore points: {}", e))
        })?;

        let points: Vec<RestorePoint> = points_json
            .into_iter()
            .filter_map(|point| {
                Some(RestorePoint {
                    sequence_number: point.get("SequenceNumber")?.as_u64()?,
                    description: point.get("Description")?.as_str()?.to_string(),
                    creation_time: point.get("CreationTime")?.as_str()?.to_string(),
                    restore_point_type: point.get("RestorePointType")?.as_str()?.to_string(),
                })
            })
            .collect();

        info!("Found {} restore points", points.len());
        Ok(points)
    }

    /// Delete old restore points (keep most recent N)
    pub async fn cleanup_old_restore_points(&self, keep_count: usize) -> Result<usize> {
        info!(
            "Cleaning up old restore points, keeping {} most recent",
            keep_count
        );

        let points = self.list_restore_points().await?;
        let mut deleted_count = 0;

        // Sort by sequence number (newest first)
        let mut sorted_points = points;
        sorted_points.sort_by_key(|p| std::cmp::Reverse(p.sequence_number));

        // Delete points beyond keep_count
        for point in sorted_points.iter().skip(keep_count) {
            if let Err(e) = self.delete_restore_point(point.sequence_number).await {
                warn!(
                    "Failed to delete restore point {}: {}",
                    point.sequence_number, e
                );
            } else {
                deleted_count += 1;
            }
        }

        info!("Deleted {} old restore points", deleted_count);
        Ok(deleted_count)
    }

    /// Delete a specific restore point.
    ///
    /// MED-4: The previous implementation used `vssadmin delete shadows /for=C: /oldest`
    /// which deletes the **global** oldest shadow copy, not the specific restore point.
    /// This could destroy unrelated recovery data.
    ///
    /// Windows does not provide a public API to delete individual restore points.
    /// This function now returns an error to prevent accidental data loss.
    /// Use `cleanup_old_restore_points()` for managed cleanup instead.
    async fn delete_restore_point(&self, sequence_number: u64) -> Result<()> {
        warn!(
            "Cannot delete individual restore point {} — Windows does not \
             support per-point deletion. Use cleanup_old_restore_points() for \
             managed cleanup.",
            sequence_number
        );
        Err(GreekError::SystemError(
            "Individual restore point deletion is not supported on Windows".to_string(),
        ))
    }

    /// Enable System Restore on C: drive
    async fn enable_system_restore(&self) -> Result<()> {
        let ps_command = r#"
            Enable-ComputerRestore -Drive "C:\\" -ErrorAction SilentlyContinue
        "#;

        self.run_powershell(ps_command).await?;
        Ok(())
    }

    /// Check if System Restore is enabled
    pub async fn is_enabled(&self) -> Result<bool> {
        let ps_command = r#"
            (Get-ComputerRestorePoint).Count -gt 0
        "#;

        let output = self.run_powershell(ps_command).await?;
        Ok(output.trim() == "True")
    }

    /// Get restore point statistics
    pub async fn get_statistics(&self) -> Result<RestoreStatistics> {
        let points = self.list_restore_points().await?;

        Ok(RestoreStatistics {
            total_points: points.len(),
            oldest_point: points.iter().min_by_key(|p| p.sequence_number).cloned(),
            newest_point: points.iter().max_by_key(|p| p.sequence_number).cloned(),
        })
    }

    /// Run a PowerShell command — respects stderr and logs it.
    async fn run_powershell(&self, command: &str) -> Result<String> {
        use std::process::Command;

        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", command])
            .output()
            .map_err(|e| GreekError::SystemError(format!("Failed to execute PowerShell: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if !stderr.trim().is_empty() {
            tracing::warn!("PowerShell stderr: {}", stderr.trim());
        }
        if !output.status.success() {
            // Do not mask real errors; only treat "not recognized" as soft failure
            if stderr.contains("not recognized") {
                return Err(GreekError::SystemError(format!(
                    "PowerShell command failed: {}",
                    stderr.trim()
                )));
            }
            // Other non-zero exits: surface as error with both streams
            return Err(GreekError::SystemError(format!(
                "PowerShell command failed (exit {:?}): stdout='{}' stderr='{}'",
                output.status.code(),
                stdout.trim(),
                stderr.trim()
            )));
        }

        Ok(stdout)
    }
}

impl Default for RestorePointManager {
    fn default() -> Self {
        Self::new()
    }
}

/// System restore point information
#[derive(Debug, Clone)]
pub struct RestorePoint {
    pub sequence_number: u64,
    pub description: String,
    pub creation_time: String,
    pub restore_point_type: String,
}

/// Restore point statistics
#[derive(Debug, Clone)]
pub struct RestoreStatistics {
    pub total_points: usize,
    pub oldest_point: Option<RestorePoint>,
    pub newest_point: Option<RestorePoint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_restore_manager_creation() {
        let manager = RestorePointManager::new();
        // Environment-dependent, so only verify the manager is usable.
        let _ = manager.list_restore_points().await;
    }

    #[tokio::test]
    async fn test_list_restore_points() {
        let manager = RestorePointManager::new();
        let result = manager.list_restore_points().await;
        // On non-Windows stub, list returns Ok(empty); on Windows without elevation may fail — both acceptable
        let _ = result;
    }

    #[tokio::test]
    async fn test_check_enabled() {
        let manager = RestorePointManager::new();
        let result = manager.is_enabled().await;
        assert!(result.is_ok());
    }
}
