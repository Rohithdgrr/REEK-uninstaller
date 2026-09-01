// Uninstaller module for removing applications

use async_trait::async_trait;
use greek_common::{
    GreekError, InstalledApp, Result, UninstallError, UninstallOptions, UninstallResult,
    UninstallStrategy,
};
use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use tracing;

/// Base uninstall strategy with common functionality
pub struct BaseUninstallStrategy {
    strategy_id: &'static str,
}

impl BaseUninstallStrategy {
    pub fn new(strategy_id: &'static str) -> Self {
        Self { strategy_id }
    }
}

#[async_trait]
impl UninstallStrategy for BaseUninstallStrategy {
    fn strategy_id(&self) -> &'static str {
        self.strategy_id
    }

    fn can_handle(&self, _app: &InstalledApp) -> bool {
        false
    }

    async fn uninstall(
        &self,
        _app: &InstalledApp,
        _options: UninstallOptions,
    ) -> Result<UninstallResult> {
        Err(GreekError::UninstallError(
            UninstallError::NoStrategyFound.to_string(),
        ))
    }

    async fn uninstall_silent(
        &self,
        _app: &InstalledApp,
        _options: UninstallOptions,
    ) -> Result<UninstallResult> {
        Err(GreekError::UninstallError(
            UninstallError::NoStrategyFound.to_string(),
        ))
    }
}

/// Standard uninstall strategy using official uninstaller
pub struct StandardUninstallStrategy {
    base: BaseUninstallStrategy,
}

impl Default for StandardUninstallStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl StandardUninstallStrategy {
    pub fn new() -> Self {
        Self {
            base: BaseUninstallStrategy::new("standard"),
        }
    }
}

#[async_trait]
impl UninstallStrategy for StandardUninstallStrategy {
    fn strategy_id(&self) -> &'static str {
        self.base.strategy_id()
    }

    fn can_handle(&self, app: &InstalledApp) -> bool {
        app.uninstall_string.is_some()
    }

    async fn uninstall(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult> {
        let uninstall_string = app.uninstall_string.as_ref().ok_or_else(|| {
            GreekError::UninstallError(UninstallError::NoStrategyFound.to_string())
        })?;

        tracing::info!("Executing standard uninstall for: {}", app.name);

        let start_time = std::time::Instant::now();
        let timeout_secs = options.timeout_seconds.unwrap_or(300);

        let result = self
            .execute_uninstall_command(uninstall_string, false, timeout_secs)
            .await?;

        let uninstall_result = UninstallResult {
            app_id: app.id,
            success: result.success,
            strategy_used: self.strategy_id().to_string(),
            exit_code: result.exit_code,
            duration: start_time.elapsed(),
            stdout: result.stdout,
            stderr: result.stderr,
            ..Default::default()
        };

        tracing::info!(
            "Uninstall completed for: {} - Success: {}",
            app.name,
            uninstall_result.success
        );

        Ok(uninstall_result)
    }

    async fn uninstall_silent(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult> {
        let uninstall_string = app
            .quiet_uninstall_string
            .as_ref()
            .or(app.uninstall_string.as_ref())
            .ok_or_else(|| {
                GreekError::UninstallError(UninstallError::NoStrategyFound.to_string())
            })?;

        tracing::info!("Executing silent uninstall for: {}", app.name);

        let start_time = std::time::Instant::now();
        let timeout_secs = options.timeout_seconds.unwrap_or(300);

        let result = self
            .execute_uninstall_command(uninstall_string, true, timeout_secs)
            .await?;

        let uninstall_result = UninstallResult {
            app_id: app.id,
            success: result.success,
            strategy_used: self.strategy_id().to_string(),
            exit_code: result.exit_code,
            duration: start_time.elapsed(),
            stdout: result.stdout,
            stderr: result.stderr,
            ..Default::default()
        };

        tracing::info!(
            "Silent uninstall completed for: {} - Success: {}",
            app.name,
            uninstall_result.success
        );

        Ok(uninstall_result)
    }
}

impl StandardUninstallStrategy {
    async fn execute_uninstall_command(
        &self,
        command: &str,
        _silent: bool,
        timeout_secs: u64,
    ) -> Result<CommandExecutionResult> {
        // Parse the command string
        let parts = self.parse_command_string(command);

        if parts.is_empty() {
            return Err(GreekError::UninstallError(
                UninstallError::ExecutionFailed("Empty command".to_string()).to_string(),
            ));
        }

        let program = parts[0].clone();
        let args: Vec<String> = parts[1..].to_vec();

        // Execute the command
        let output = timeout(
            Duration::from_secs(timeout_secs),
            tokio::task::spawn_blocking(move || Command::new(&program).args(&args).output()),
        )
        .await
        .map_err(|_| GreekError::UninstallError(UninstallError::Timeout(timeout_secs).to_string()))?
        .map_err(|e| {
            GreekError::UninstallError(UninstallError::ExecutionFailed(e.to_string()).to_string())
        })?
        .map_err(|e| {
            GreekError::UninstallError(UninstallError::ExecutionFailed(e.to_string()).to_string())
        })?;

        let success = output.status.success();
        let exit_code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandExecutionResult {
            success,
            exit_code,
            stdout: Some(stdout),
            stderr: Some(stderr),
        })
    }

    fn parse_command_string(&self, command: &str) -> Vec<String> {
        // CR-8: Handle edge cases in Windows uninstall command strings.
        //
        // Uninstall strings can look like:
        //   "C:\Program Files\App\uninstall.exe" /S
        //   msiexec.exe /x {GUID} /qn
        //   "C:\App\uninst.exe" /S /D="C:\My Path"
        //
        // The hand-rolled parser handles the common cases. For complex
        // inputs (escaped quotes, trailing backslashes before quotes),
        // we fall back to shell_words then split_whitespace.
        if command.contains('"') {
            let mut parts = Vec::new();
            let mut current = String::new();
            let mut in_quotes = false;
            let chars: Vec<char> = command.chars().collect();
            let len = chars.len();
            let mut i = 0;
            while i < len {
                let c = chars[i];
                match c {
                    '"' => {
                        // CR-8: handle \" (escaped quote inside quoted string)
                        if in_quotes && i + 1 < len && chars[i + 1] == '"' {
                            current.push('"');
                            i += 1; // skip the escaped quote
                        } else {
                            in_quotes = !in_quotes;
                        }
                    }
                    ' ' if !in_quotes => {
                        if !current.is_empty() {
                            parts.push(std::mem::take(&mut current));
                        }
                    }
                    _ => current.push(c),
                }
                i += 1;
            }
            if !current.is_empty() {
                parts.push(current);
            }
            if !parts.is_empty() {
                return parts;
            }
        }
        shell_words::split(command)
            .unwrap_or_else(|_| command.split_whitespace().map(|s| s.to_string()).collect())
    }
}

struct CommandExecutionResult {
    success: bool,
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
}

/// MSI uninstall strategy
pub struct MsiUninstallStrategy {
    base: BaseUninstallStrategy,
}

impl Default for MsiUninstallStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl MsiUninstallStrategy {
    pub fn new() -> Self {
        Self {
            base: BaseUninstallStrategy::new("msi"),
        }
    }
}

#[async_trait]
impl UninstallStrategy for MsiUninstallStrategy {
    fn strategy_id(&self) -> &'static str {
        self.base.strategy_id()
    }

    fn can_handle(&self, app: &InstalledApp) -> bool {
        app.uninstall_string
            .as_ref()
            .map(|s| s.to_lowercase().contains("msiexec"))
            .unwrap_or(false)
    }

    async fn uninstall(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult> {
        let product_code = self.extract_product_code(app)?;

        tracing::info!("Executing MSI uninstall for: {}", app.name);

        let start_time = std::time::Instant::now();
        let timeout_secs = options.timeout_seconds.unwrap_or(300);

        let output = timeout(
            Duration::from_secs(timeout_secs),
            tokio::task::spawn_blocking(move || {
                Command::new("msiexec.exe")
                    .args(["/x", &product_code])
                    .output()
            }),
        )
        .await
        .map_err(|_| GreekError::UninstallError(UninstallError::Timeout(timeout_secs).to_string()))?
        .map_err(|e| {
            GreekError::UninstallError(UninstallError::ExecutionFailed(e.to_string()).to_string())
        })?
        .map_err(|e| {
            GreekError::UninstallError(UninstallError::ExecutionFailed(e.to_string()).to_string())
        })?;

        let success = output.status.success();

        Ok(UninstallResult {
            app_id: app.id,
            success,
            strategy_used: self.strategy_id().to_string(),
            exit_code: output.status.code(),
            duration: start_time.elapsed(),
            stdout: Some(String::from_utf8_lossy(&output.stdout).to_string()),
            stderr: Some(String::from_utf8_lossy(&output.stderr).to_string()),
            ..Default::default()
        })
    }

    async fn uninstall_silent(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult> {
        let product_code = self.extract_product_code(app)?;

        tracing::info!("Executing MSI silent uninstall for: {}", app.name);

        let start_time = std::time::Instant::now();
        let timeout_secs = options.timeout_seconds.unwrap_or(300);

        let output = timeout(
            Duration::from_secs(timeout_secs),
            tokio::task::spawn_blocking(move || {
                Command::new("msiexec.exe")
                    .args(["/x", &product_code, "/qn", "/norestart"])
                    .output()
            }),
        )
        .await
        .map_err(|_| GreekError::UninstallError(UninstallError::Timeout(timeout_secs).to_string()))?
        .map_err(|e| {
            GreekError::UninstallError(UninstallError::ExecutionFailed(e.to_string()).to_string())
        })?
        .map_err(|e| {
            GreekError::UninstallError(UninstallError::ExecutionFailed(e.to_string()).to_string())
        })?;

        let success = output.status.success();

        Ok(UninstallResult {
            app_id: app.id,
            success,
            strategy_used: format!("{}-silent", self.strategy_id()),
            exit_code: output.status.code(),
            duration: start_time.elapsed(),
            stdout: Some(String::from_utf8_lossy(&output.stdout).to_string()),
            stderr: Some(String::from_utf8_lossy(&output.stderr).to_string()),
            ..Default::default()
        })
    }
}

impl MsiUninstallStrategy {
    fn extract_product_code(&self, app: &InstalledApp) -> Result<String> {
        use std::sync::OnceLock;
        static MSI_RE: OnceLock<regex::Regex> = OnceLock::new();
        let re = MSI_RE.get_or_init(|| {
            regex::Regex::new(
                r"\{[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}\}",
            )
            .expect("valid MSI regex")
        });

        let uninstall_string = app.uninstall_string.as_ref().ok_or_else(|| {
            GreekError::UninstallError(UninstallError::NoStrategyFound.to_string())
        })?;

        if let Some(captures) = re.captures(uninstall_string) {
            Ok(captures[0].to_string())
        } else {
            Err(GreekError::UninstallError(
                UninstallError::ExecutionFailed("Could not extract MSI product code".to_string())
                    .to_string(),
            ))
        }
    }
}

/// Force remove strategy - deletes files and registry directly
pub struct ForceRemoveStrategy {
    base: BaseUninstallStrategy,
}

impl Default for ForceRemoveStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ForceRemoveStrategy {
    pub fn new() -> Self {
        Self {
            base: BaseUninstallStrategy::new("force-remove"),
        }
    }
}

#[async_trait]
impl UninstallStrategy for ForceRemoveStrategy {
    fn strategy_id(&self) -> &'static str {
        self.base.strategy_id()
    }

    fn can_handle(&self, _app: &InstalledApp) -> bool {
        true // Force remove can handle anything
    }

    async fn uninstall(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult> {
        tracing::warn!("Executing force remove for: {}", app.name);

        let start_time = std::time::Instant::now();
        let mut result = UninstallResult {
            app_id: app.id,
            strategy_used: self.strategy_id().to_string(),
            ..Default::default()
        };

        // Record a rollback-able transaction before any destructive action so
        // the uninstall can be undone.
        let mut transaction = crate::backup::UninstallTransaction::new(&app.name)?;

        // Kill processes
        if options.kill_processes {
            if let Some(ref location) = app.install_location {
                let killed = self.kill_processes_by_path(location).await?;
                tracing::info!("Killed {} processes", killed);
            }
        }

        // Delete files
        if let Some(ref location) = app.install_location {
            // V001: refuse to force-remove protected system paths
            let protected = greek_common::PROTECTED_PATHS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            if crate::utils::is_protected_path(location, &protected) {
                return Err(GreekError::SafetyError(format!(
                    "Refusing to force-remove protected path for app: {}",
                    app.name
                )));
            }

            if location.exists() {
                // Back up the file tree before deletion so it can be restored.
                if let Err(e) = transaction.add_file_or_dir(location) {
                    tracing::warn!("Failed to back up {}: {}", location.display(), e);
                }
                if options.move_to_recycle_bin {
                    crate::utils::move_to_recycle_bin(location)?;
                } else {
                    crate::utils::delete_directory(location)?;
                }
                result.files_deleted.push(location.clone());
            }
        }

        // Delete registry keys
        for key in &app.registry_keys {
            if let Err(e) = transaction.add_registry_key(&key.path) {
                tracing::warn!("Failed to back up registry key {}: {}", key.path, e);
            }
            match crate::utils::delete_registry_key(&key.path) {
                Ok(()) => {
                    result.registry_keys_deleted.push(key.path.clone());
                    tracing::info!("Deleted registry key: {}", key.path);
                }
                Err(e) => {
                    tracing::warn!("Failed to delete registry key {}: {}", key.path, e);
                }
            }
        }

        // Persist the manifest so the transaction survives a restart.
        if let Err(e) = transaction.save_manifest() {
            tracing::warn!("Failed to save backup manifest: {}", e);
        } else if !transaction.entries.is_empty() {
            result.backup_id = Some(transaction.id);
            tracing::info!(
                "Backup transaction {} recorded for {} ({} items)",
                transaction.id,
                app.name,
                transaction.entries.len()
            );
        }

        result.success = true;
        result.duration = start_time.elapsed();

        tracing::warn!("Force remove completed for: {}", app.name);

        Ok(result)
    }

    async fn uninstall_silent(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult> {
        // Force remove is always silent
        self.uninstall(app, options).await
    }
}

impl ForceRemoveStrategy {
    async fn kill_processes_by_path(&self, path: &std::path::Path) -> Result<usize> {
        use sysinfo::System;

        fn norm(s: &str) -> String {
            s.to_lowercase()
                .replace('\\', "/")
                .trim_end_matches('/')
                .to_string()
        }
        // Reuse one System instance per call; refresh only processes
        let mut system = System::new_all();
        system.refresh_all();

        let mut killed_count = 0;
        let path_norm = norm(&path.to_string_lossy());

        for (pid, process) in system.processes() {
            if let Some(exe_path) = process.exe() {
                let exe_norm = norm(&exe_path.to_string_lossy());
                let is_child =
                    exe_norm == path_norm || exe_norm.starts_with(&(path_norm.clone() + "/"));
                if is_child && process.kill() {
                    killed_count += 1;
                    // Cap log length to avoid leaking long paths
                    tracing::info!(
                        "Killed process: {} ({})",
                        pid,
                        &exe_norm[..exe_norm.len().min(120)]
                    );
                }
            }
        }

        Ok(killed_count)
    }
}

/// Uninstaller manager that coordinates multiple strategies
pub struct UninstallerManager {
    strategies: Vec<Box<dyn UninstallStrategy>>,
}

impl UninstallerManager {
    pub fn new() -> Self {
        let mut manager = Self {
            strategies: Vec::new(),
        };

        // Register default strategies
        manager.register_strategy(Box::new(MsiUninstallStrategy::new()));
        manager.register_strategy(Box::new(StandardUninstallStrategy::new()));
        manager.register_strategy(Box::new(ForceRemoveStrategy::new()));

        manager
    }

    pub fn register_strategy(&mut self, strategy: Box<dyn UninstallStrategy>) {
        self.strategies.push(strategy);
    }

    pub async fn uninstall(
        &self,
        app: &InstalledApp,
        options: UninstallOptions,
    ) -> Result<UninstallResult> {
        // Find the best strategy
        let strategy = self
            .strategies
            .iter()
            .find(|s| s.can_handle(app))
            .ok_or_else(|| {
                GreekError::UninstallError(UninstallError::NoStrategyFound.to_string())
            })?;

        // Execute uninstallation
        if options.silent {
            strategy.uninstall_silent(app, options).await
        } else {
            strategy.uninstall(app, options).await
        }
    }
}

impl Default for UninstallerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use greek_common::{InstallSource, RegistryHive};

    #[tokio::test]
    async fn test_standard_strategy() {
        let strategy = StandardUninstallStrategy::new();

        let mut app = InstalledApp::new(
            "Test App".to_string(),
            InstallSource::Registry {
                hive: RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        );
        app.uninstall_string = Some("notepad.exe".to_string());

        assert!(strategy.can_handle(&app));
    }

    #[tokio::test]
    async fn test_msi_strategy() {
        let strategy = MsiUninstallStrategy::new();

        let mut app = InstalledApp::new(
            "Test App".to_string(),
            InstallSource::Registry {
                hive: RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        );
        app.uninstall_string =
            Some("MsiExec.exe /X{12345678-1234-1234-1234-123456789012}".to_string());

        assert!(strategy.can_handle(&app));
    }

    #[tokio::test]
    async fn test_force_strategy() {
        let strategy = ForceRemoveStrategy::new();

        let app = InstalledApp::new(
            "Test App".to_string(),
            InstallSource::Registry {
                hive: RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        );

        assert!(strategy.can_handle(&app));
    }

    #[test]
    fn test_parse_command_quoted_path() {
        let strategy = StandardUninstallStrategy::new();

        // Quoted path with spaces must stay one token
        let parts =
            strategy.parse_command_string(r#""C:\Program Files\App\uninstall.exe" /S /noreboot"#);
        assert_eq!(
            parts,
            vec![r"C:\Program Files\App\uninstall.exe", "/S", "/noreboot",]
        );

        // Unquoted simple command
        let parts = strategy.parse_command_string("msiexec.exe /x {GUID} /qn");
        assert_eq!(parts, vec!["msiexec.exe", "/x", "{GUID}", "/qn"]);

        // CR-8: command with quoted argument containing spaces
        let parts = strategy.parse_command_string(r#""C:\App\uninst.exe" /S /D="C:\My Path""#);
        assert_eq!(parts, vec![r"C:\App\uninst.exe", "/S", r"/D=C:\My Path",]);
    }
}
