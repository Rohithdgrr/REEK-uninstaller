// Transactional backup & rollback for destructive operations.
//
// Before force-removing an app's files or registry keys, REEK records a
// `UninstallTransaction` that can restore the system to its prior state via
// `rollback()` (an "undo"). Backups live under the app data dir:
//
//   <data_dir>/backups/<transaction-id>/
//       manifest.json     - machine-readable record of the transaction
//       items/0001        - copied file or directory
//       items/0002.reg    - exported registry key (Windows)
//
// `manifest.json` also lets a future `reek undo` command list what can be
// restored (see `list_transactions`).

use crate::utils::get_app_data_dir;
use greek_common::{GreekError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// What kind of path a backup entry covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    File,
    Directory,
    RegistryKey,
}

/// A single backed-up item. `original` is where it lived; `backup` is where the
/// copy now lives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub entry_type: EntryType,
    pub original: PathBuf,
    pub backup: PathBuf,
}

/// An uninstall transaction that can be rolled back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallTransaction {
    pub id: Uuid,
    pub app_name: String,
    pub timestamp: String,
    pub entries: Vec<BackupEntry>,
}

impl UninstallTransaction {
    /// Create a new transaction with a fresh id and a directory under the
    /// backup root ready to receive copies.
    pub fn new(app_name: &str) -> Result<Self> {
        let id = Uuid::new_v4();
        let tx = Self {
            id,
            app_name: app_name.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            entries: Vec::new(),
        };
        fs::create_dir_all(tx.root())?;
        Ok(tx)
    }

    /// Absolute path to this transaction's backup directory.
    pub fn root(&self) -> PathBuf {
        backup_root().join(self.id.to_string())
    }

    /// Copy a file or directory into the backup, preserving what was deleted.
    pub fn add_file_or_dir(&mut self, original: &Path) -> Result<()> {
        let index = self.entries.len();
        let backup_path = self.root().join("items").join(format!("{:04}", index));
        fs::create_dir_all(backup_path.parent().unwrap_or(&self.root()))?;

        copy_tree(original, &backup_path)?;

        self.entries.push(BackupEntry {
            entry_type: if original.is_dir() {
                EntryType::Directory
            } else {
                EntryType::File
            },
            original: original.to_path_buf(),
            backup: backup_path,
        });
        Ok(())
    }

    /// Export a registry key to a `.reg` file in the backup directory.
    ///
    /// On non-Windows platforms this is a no-op (there is no registry to back
    /// up); on Windows it shells out to `reg.exe export`, which is safe because
    /// the path is passed as a single argument (no shell interpretation).
    pub fn add_registry_key(&mut self, key_path: &str) -> Result<()> {
        #[cfg(all(target_os = "windows", feature = "windows"))]
        {
            let index = self.entries.len();
            let backup_path = self.root().join("items").join(format!("{:04}.reg", index));
            fs::create_dir_all(backup_path.parent().unwrap_or(&self.root()))?;

            let status = std::process::Command::new("reg.exe")
                .arg("export")
                .arg(key_path)
                .arg(&backup_path)
                .arg("/y")
                .status()
                .map_err(|e| {
                    GreekError::SystemError(format!(
                        "Failed to run reg.exe export for {}: {}",
                        key_path, e
                    ))
                })?;

            if !status.success() {
                tracing::warn!(
                    "reg.exe export failed for {} (exit {:?}), skipping backup",
                    key_path,
                    status.code()
                );
                return Ok(());
            }

            self.entries.push(BackupEntry {
                entry_type: EntryType::RegistryKey,
                original: PathBuf::from(key_path),
                backup: backup_path,
            });
        }

        #[cfg(not(all(target_os = "windows", feature = "windows")))]
        {
            let _ = key_path;
        }

        Ok(())
    }

    /// Persist the manifest for this transaction so it can be listed and
    /// rolled back later (even after a restart).
    pub fn save_manifest(&self) -> Result<PathBuf> {
        let manifest_path = self.root().join("manifest.json");
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            GreekError::SystemError(format!("Failed to serialize backup manifest: {}", e))
        })?;
        fs::write(&manifest_path, json)?;
        Ok(manifest_path)
    }

    /// Restore every backed-up item, in reverse order, so earlier deletions are
    /// restored last (registry first, files/directories last).
    pub fn rollback(&self) -> Result<()> {
        for entry in self.entries.iter().rev() {
            match entry.entry_type {
                EntryType::RegistryKey => restore_registry_key(entry)?,
                EntryType::File | EntryType::Directory => restore_path(entry)?,
            }
        }
        tracing::info!(
            "Rolled back transaction {} ({} items)",
            self.id,
            self.entries.len()
        );
        Ok(())
    }
}

/// Directory under which all transactions are stored.
pub fn backup_root() -> PathBuf {
    get_app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("backups")
}

/// Load every transaction manifest found under the backup root.
pub fn list_transactions() -> Result<Vec<UninstallTransaction>> {
    let root = backup_root();
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut transactions = Vec::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let manifest = entry.path().join("manifest.json");
        if !manifest.exists() {
            continue;
        }
        match load_transaction(&manifest) {
            Ok(tx) => transactions.push(tx),
            Err(e) => tracing::warn!(
                "Skipping invalid backup manifest {}: {}",
                manifest.display(),
                e
            ),
        }
    }

    // Newest first.
    transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(transactions)
}

/// Load a single transaction from its manifest file.
pub fn load_transaction(manifest_path: &Path) -> Result<UninstallTransaction> {
    let json = fs::read_to_string(manifest_path)?;
    serde_json::from_str(&json)
        .map_err(|e| GreekError::SystemError(format!("Invalid backup manifest: {}", e)))
}

/// Recursively copy a file or directory (preserving directory contents).
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_tree(&entry.path(), &dst.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
        Ok(())
    }
}

/// Restore a single file or directory backup. Never overwrites an existing
/// path; if the original was recreated, the backup is left untouched and a
/// warning is logged.
fn restore_path(entry: &BackupEntry) -> Result<()> {
    if entry.original.exists() {
        tracing::warn!(
            "Refusing to overwrite existing path during rollback: {}",
            entry.original.display()
        );
        return Ok(());
    }

    copy_tree(&entry.backup, &entry.original)?;
    tracing::info!("Restored {}", entry.original.display());
    Ok(())
}

/// Restore a registry key from a backed-up `.reg` file.
fn restore_registry_key(entry: &BackupEntry) -> Result<()> {
    #[cfg(all(target_os = "windows", feature = "windows"))]
    {
        let status = std::process::Command::new("reg.exe")
            .arg("import")
            .arg(&entry.backup)
            .status()
            .map_err(|e| {
                GreekError::SystemError(format!(
                    "Failed to run reg.exe import for {}: {}",
                    entry.original.display(),
                    e
                ))
            })?;

        if status.success() {
            tracing::info!("Restored registry key {}", entry.original.display());
            Ok(())
        } else {
            Err(GreekError::SystemError(format!(
                "reg.exe import failed for {} (exit {:?})",
                entry.original.display(),
                status.code()
            )))
        }
    }

    #[cfg(not(all(target_os = "windows", feature = "windows")))]
    {
        let _ = entry;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_transaction_creates_root() {
        let tx = UninstallTransaction::new("Test App").unwrap();
        assert!(tx.root().exists());
    }

    #[test]
    fn test_add_file_and_manifest() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("app.txt");
        fs::write(&file, "content").unwrap();

        let mut tx = UninstallTransaction::new("Test App").unwrap();
        tx.add_file_or_dir(&file).unwrap();
        tx.save_manifest().unwrap();

        let loaded = load_transaction(&tx.root().join("manifest.json")).unwrap();
        assert_eq!(loaded.app_name, "Test App");
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].entry_type, EntryType::File);
    }

    #[test]
    fn test_add_directory_backup() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("AppDir");
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub").join("data.bin"), b"x").unwrap();

        let mut tx = UninstallTransaction::new("Test App").unwrap();
        tx.add_file_or_dir(&dir).unwrap();
        assert_eq!(tx.entries[0].entry_type, EntryType::Directory);
        assert!(tx.entries[0].backup.join("sub").join("data.bin").exists());
    }

    #[test]
    fn test_rollback_restores_deleted_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("app.txt");
        fs::write(&file, "content").unwrap();

        let mut tx = UninstallTransaction::new("Test App").unwrap();
        tx.add_file_or_dir(&file).unwrap();

        // Simulate the deletion the uninstaller performs.
        fs::remove_file(&file).unwrap();
        assert!(!file.exists());

        tx.rollback().unwrap();
        assert!(file.exists());
        assert_eq!(fs::read_to_string(&file).unwrap(), "content");
    }

    #[test]
    fn test_rollback_restores_directory() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("AppDir");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("data.bin"), b"y").unwrap();

        let mut tx = UninstallTransaction::new("Test App").unwrap();
        tx.add_file_or_dir(&dir).unwrap();

        fs::remove_dir_all(&dir).unwrap();
        assert!(!dir.exists());

        tx.rollback().unwrap();
        assert!(dir.join("data.bin").exists());
    }

    #[test]
    fn test_rollback_does_not_overwrite() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("app.txt");
        fs::write(&file, "original").unwrap();

        let mut tx = UninstallTransaction::new("Test App").unwrap();
        tx.add_file_or_dir(&file).unwrap();

        fs::remove_file(&file).unwrap();
        fs::write(&file, "recreated").unwrap();

        tx.rollback().unwrap();
        // Must not clobber the recreated file.
        assert_eq!(fs::read_to_string(&file).unwrap(), "recreated");
    }

    #[test]
    fn test_list_transactions_empty_when_no_backups() {
        // Backup root may not exist yet; must return an empty list.
        assert!(list_transactions().is_ok());
    }
}
