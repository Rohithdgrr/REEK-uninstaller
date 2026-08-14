// Leftover detection module for finding orphaned artifacts

use async_trait::async_trait;
use greek_common::{
    ArtifactType, InstalledApp, LeftoverAnalyzer, LeftoverArtifact, Result, SafetyLevel,
};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Base leftover analyzer with common functionality
pub struct BaseLeftoverAnalyzer {
    analyzer_id: &'static str,
}

impl BaseLeftoverAnalyzer {
    pub fn new(analyzer_id: &'static str) -> Self {
        Self { analyzer_id }
    }
}

#[async_trait]
impl LeftoverAnalyzer for BaseLeftoverAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        self.analyzer_id
    }

    async fn analyze(&self, _app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        Ok(Vec::new())
    }

    async fn analyze_system(&self) -> Result<Vec<LeftoverArtifact>> {
        Ok(Vec::new())
    }

    fn score_confidence(&self, _artifact: &LeftoverArtifact, _app: &InstalledApp) -> f32 {
        0.5
    }
}

/// File system leftover analyzer
pub struct FileSystemLeftoverAnalyzer {
    base: BaseLeftoverAnalyzer,
    scan_directories: Vec<PathBuf>,
}

impl FileSystemLeftoverAnalyzer {
    pub fn new(scan_directories: Vec<PathBuf>) -> Self {
        Self {
            base: BaseLeftoverAnalyzer::new("filesystem"),
            scan_directories,
        }
    }
}

#[async_trait]
impl LeftoverAnalyzer for FileSystemLeftoverAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        self.base.analyzer_id()
    }

    async fn analyze(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        let mut artifacts = Vec::new();

        // Search for directories/files that might belong to this app
        let app_name_lower = app.name.to_lowercase();
        let publisher_lower = app
            .publisher
            .as_ref()
            .map(|p| p.to_lowercase())
            .unwrap_or_default();

        for directory in &self.scan_directories {
            if !directory.exists() {
                continue;
            }

            let found = self
                .scan_directory_for_app(directory, &app_name_lower, &publisher_lower)
                .await?;
            artifacts.extend(found);
        }

        // Score each artifact
        for artifact in &mut artifacts {
            artifact.confidence = self.score_confidence(artifact, app);
            artifact.safety_level = self.determine_safety_level(artifact);
        }

        Ok(artifacts)
    }

    async fn analyze_system<'a>(&'a self) -> Result<Vec<LeftoverArtifact>> {
        let mut artifacts = Vec::new();

        // Scan for orphaned directories
        for directory in &self.scan_directories {
            if !directory.exists() {
                continue;
            }

            let found = self.scan_for_orphans(directory).await?;
            artifacts.extend(found);
        }

        Ok(artifacts)
    }

    fn score_confidence(&self, artifact: &LeftoverArtifact, app: &InstalledApp) -> f32 {
        let path_str = artifact.path.to_string_lossy().to_lowercase();
        let app_name_lower = app.name.to_lowercase();
        let publisher_lower = app
            .publisher
            .as_ref()
            .map(|p| p.to_lowercase())
            .unwrap_or_default();

        let mut score = 0.0;

        // Exact name match
        if path_str.contains(&app_name_lower) {
            score += 0.4;
        }

        // Publisher match
        if !publisher_lower.is_empty() && path_str.contains(&publisher_lower) {
            score += 0.3;
        }

        // Check if path matches install location
        if let Some(ref install_location) = app.install_location {
            let install_str = install_location.to_string_lossy().to_lowercase();
            if path_str.starts_with(&install_str) {
                score += 0.3;
            }
        }

        // Check for common app directories
        if path_str.contains("appdata") || path_str.contains("program data") {
            score += 0.1;
        }

        (score as f32).min(1.0)
    }
}

impl FileSystemLeftoverAnalyzer {
    async fn scan_directory_for_app(
        &self,
        directory: &PathBuf,
        app_name: &str,
        publisher: &str,
    ) -> Result<Vec<LeftoverArtifact>> {
        let mut artifacts = Vec::new();

        for entry in WalkDir::new(directory)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Check if path contains app name or publisher
            let path_str = path.to_string_lossy().to_lowercase();

            if path_str.contains(app_name)
                || (!publisher.is_empty() && path_str.contains(publisher))
            {
                let artifact_type = if path.is_dir() {
                    ArtifactType::Directory
                } else {
                    ArtifactType::File
                };

                let mut artifact = LeftoverArtifact::new(artifact_type, path.to_path_buf());
                artifact.description = format!("Potential leftover for {}", app_name);

                // Get size
                if let Ok(metadata) = std::fs::metadata(path) {
                    artifact.size_bytes = Some(metadata.len());
                }

                artifacts.push(artifact);
            }
        }

        Ok(artifacts)
    }

    async fn scan_for_orphans(&self, directory: &PathBuf) -> Result<Vec<LeftoverArtifact>> {
        let mut artifacts = Vec::new();

        // This is a simplified orphan detection
        // In production, this would cross-reference with installed apps
        for entry in WalkDir::new(directory)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Look for directories that look like app directories but might be orphaned
            if path.is_dir() {
                if let Some(dir_name) = path.file_name() {
                    let dir_name_str = dir_name.to_string_lossy();

                    // Heuristic: directory with app-like name that hasn't been modified recently
                    if let Ok(metadata) = std::fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            let datetime: chrono::DateTime<chrono::Utc> =
                                chrono::DateTime::from(modified);
                            let age = chrono::Utc::now().signed_duration_since(datetime);

                            // If older than 30 days, might be orphaned
                            if age.num_days() > 30 {
                                let mut artifact = LeftoverArtifact::new(
                                    ArtifactType::Directory,
                                    path.to_path_buf(),
                                );
                                artifact.confidence = 0.3; // Low confidence for orphan detection
                                artifact.safety_level = SafetyLevel::Caution;
                                artifact.description =
                                    format!("Potential orphan directory: {}", dir_name_str);
                                artifact.size_bytes = Some(metadata.len());

                                artifacts.push(artifact);
                            }
                        }
                    }
                }
            }
        }

        Ok(artifacts)
    }

    fn determine_safety_level(&self, artifact: &LeftoverArtifact) -> SafetyLevel {
        let path_str = artifact.path.to_string_lossy().to_lowercase();

        // System directories are dangerous
        if path_str.contains("windows") || path_str.contains("system32") {
            return SafetyLevel::Critical;
        }

        // Program files are dangerous
        if path_str.contains("program files") {
            return SafetyLevel::Dangerous;
        }

        // AppData is generally safe for user apps
        if path_str.contains("appdata") {
            return SafetyLevel::Safe;
        }

        // Default to caution
        SafetyLevel::Caution
    }
}

/// Registry leftover analyzer (placeholder for platform-specific implementation)
#[cfg(target_os = "windows")]
pub struct RegistryLeftoverAnalyzer {
    base: BaseLeftoverAnalyzer,
}

#[cfg(target_os = "windows")]
impl Default for RegistryLeftoverAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryLeftoverAnalyzer {
    pub fn new() -> Self {
        Self {
            base: BaseLeftoverAnalyzer::new("registry"),
        }
    }
}

#[cfg(target_os = "windows")]
#[async_trait]
impl LeftoverAnalyzer for RegistryLeftoverAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        self.base.analyzer_id()
    }

    async fn analyze(&self, _app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        // This will be implemented in greek-windows crate
        Ok(Vec::new())
    }

    async fn analyze_system(&self) -> Result<Vec<LeftoverArtifact>> {
        Ok(Vec::new())
    }

    fn score_confidence(&self, _artifact: &LeftoverArtifact, _app: &InstalledApp) -> f32 {
        0.5
    }
}

/// Leftover analyzer manager
pub struct LeftoverAnalyzerManager {
    analyzers: Vec<Box<dyn LeftoverAnalyzer>>,
}

impl LeftoverAnalyzerManager {
    pub fn new() -> Self {
        Self {
            analyzers: Vec::new(),
        }
    }

    pub fn register_analyzer(&mut self, analyzer: Box<dyn LeftoverAnalyzer>) {
        self.analyzers.push(analyzer);
    }

    pub async fn analyze_app(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        let mut all_artifacts = Vec::new();

        for analyzer in &self.analyzers {
            match analyzer.analyze(app).await {
                Ok(artifacts) => all_artifacts.extend(artifacts),
                Err(e) => tracing::error!("Analyzer {} failed: {}", analyzer.analyzer_id(), e),
            }
        }

        Ok(all_artifacts)
    }
}

impl Default for LeftoverAnalyzerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use greek_common::InstallSource;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_filesystem_analyzer() {
        let temp_dir = TempDir::new().unwrap();
        let scan_dirs = vec![temp_dir.path().to_path_buf()];

        let analyzer = FileSystemLeftoverAnalyzer::new(scan_dirs);

        let app = InstalledApp::new(
            "TestApp".to_string(),
            InstallSource::Registry {
                hive: greek_common::RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        );

        let artifacts = analyzer.analyze(&app).await.unwrap();
        assert!(artifacts.is_empty());
    }

    #[test]
    fn test_confidence_scoring() {
        let analyzer = FileSystemLeftoverAnalyzer::new(vec![]);

        let app = InstalledApp::new(
            "TestApp".to_string(),
            InstallSource::Registry {
                hive: greek_common::RegistryHive::Hklm,
                key_path: "test".to_string(),
            },
        );

        let artifact = LeftoverArtifact::new(
            ArtifactType::Directory,
            PathBuf::from("C:\\Users\\Test\\TestApp"),
        );

        let score = analyzer.score_confidence(&artifact, &app);
        assert!(score > 0.0);
    }
}
