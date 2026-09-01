// Leftover detection module for finding orphaned artifacts
// Enhanced to scan all drives & standard locations (Program Files, AppData, Users, etc.)
// with accurate folder sizes and tokenized matching for apps like "Cursor Desktop".

use async_trait::async_trait;
use greek_common::{
    ArtifactType, InstalledApp, LeftoverAnalyzer, LeftoverArtifact, Result, SafetyLevel,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Extensions that must NEVER be treated as deletable leftovers / junk / duplicates.
/// User explicitly requested: don't touch documents & images (.pdf, .doc, .ppt, .xlsx, .jpg, .png, …).
/// Video scanner is separate (only video exts). This guard ensures leftover & duplicate scanners
/// never flag these as safe-to-delete, even if filename contains app token (e.g. "Report.pdf" for app "Report").
const EXCLUDED_DOC_IMAGE_EXTS: &[&str] = &[
    "pdf","doc","docx","ppt","pptx","xls","xlsx","csv","txt","rtf","odt","ods","odp",
    "jpg","jpeg","png","gif","bmp","tiff","tif","webp","svg","heic","heif","raw","ico",
    "psd","ai","eps","indd",
];

fn is_excluded_doc_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXCLUDED_DOC_IMAGE_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Return logical drives on Windows (C:\, D:\, …) by probing A-Z.
/// Lightweight vs GetLogicalDriveStringsW and works on all targets.
fn enumerate_drives() -> Vec<String> {
    let mut drives = Vec::new();
    #[cfg(target_os = "windows")]
    {
        for letter in b'A'..=b'Z' {
            let p = format!("{}:\\", letter as char);
            if Path::new(&p).exists() {
                // Filter to fixed drives where possible: skip A: / B: floppy unless present
                // Also include D:\ etc if Program Files or Users exists there.
                drives.push(p);
            }
        }
        if drives.is_empty() {
            drives.push("C:\\".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        drives.push("/".to_string());
    }
    drives
}

/// Build a comprehensive list of filesystem roots to scan for leftovers.
/// Covers every logical drive's Program Files, ProgramData, Users/AppData, and
/// optional Windows (shallow). This is what makes "Cursor" show leftovers from
/// D:\Dev, C:\Users\rohit\AppData\Local\Cursor, C:\Program Files\Cursor, etc.
pub fn build_comprehensive_scan_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |p: PathBuf| {
        if p.exists() {
            let key = p.to_string_lossy().to_lowercase();
            if seen.insert(key) {
                roots.push(p);
            }
        }
    };

    // Env-based well-known locations (exact, already resolved)
    for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramData", "LOCALAPPDATA", "APPDATA"] {
        if let Ok(v) = std::env::var(var) {
            push(PathBuf::from(v));
        }
    }
    // LocalLow is not in env on some configs
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        // Derive LocalLow sibling
        if let Some(parent) = Path::new(&local).parent() {
            push(parent.join("LocalLow"));
        }
    }
    // USERPROFILE base (covers Desktop/Documents leftovers)
    if let Ok(up) = std::env::var("USERPROFILE") {
        push(PathBuf::from(up));
    }

    // Enumerate every drive's standard program locations
    for drive in enumerate_drives() {
        let d = drive.trim_end_matches('\\').trim_end_matches('/');
        // Avoid duplicate for C:\ already added via env vars (but keep dedup)
        push(PathBuf::from(format!("{}\\Program Files", d)));
        push(PathBuf::from(format!("{}\\Program Files (x86)", d)));
        push(PathBuf::from(format!("{}\\ProgramData", d)));
        // Per-drive Users enumeration (C:\Users, D:\Users if exists)
        let users_root = PathBuf::from(format!("{}\\Users", d));
        if users_root.exists() {
            // Add Users root shallow (for top-level profiles) but prefer per-user AppData
            // Enumerate each user profile's AppData
            if let Ok(entries) = std::fs::read_dir(&users_root) {
                for ent in entries.filter_map(|e| e.ok()) {
                    let upath = ent.path();
                    if upath.is_dir() {
                        let name = upath.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        // Skip system profiles
                        if ["Public", "Default", "Default User", "All Users"].contains(&name) {
                            continue;
                        }
                        push(upath.join("AppData").join("Local"));
                        push(upath.join("AppData").join("Roaming"));
                        push(upath.join("AppData").join("LocalLow"));
                        // Also add the profile root itself shallow (covers portable installs in user dir)
                        push(upath.clone());
                    }
                }
            }
            // Keep root as fallback shallow scan (depth limited)
            push(users_root);
        }
        // Windows folder shallow (user explicitly wants Windows leftovers; depth capped in scanner)
        // Only for C:\ to avoid scanning Windows on every drive
        if d.eq_ignore_ascii_case("C:") {
            push(PathBuf::from(format!("{}\\Windows", d)));
        }
    }

    // Fallback: ensure at least ProgramData + USERPROFILE covered even if drive probe failed
    if roots.is_empty() {
        roots.push(PathBuf::from("C:\\ProgramData"));
        if let Ok(up) = std::env::var("USERPROFILE") {
            roots.push(PathBuf::from(up));
        }
    }
    roots
}

/// Tokenize an app name into searchable keywords.
/// "Cursor Desktop" -> ["cursor","desktop","cursor desktop"]
/// "Docker Desktop" -> ["docker","desktop","docker desktop"]
/// Filters tokens <3 chars to avoid false positives ("a", "an").
fn tokenize_app_name(name: &str) -> Vec<String> {
    let lower = name.to_lowercase();
    let mut tokens: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_string())
        .collect();
    // Add full normalized name if >=3 chars and not already present
    let normalized = lower.trim().to_string();
    if normalized.len() >= 3 && !tokens.contains(&normalized) {
        tokens.push(normalized.clone());
        // Also add slug without spaces for paths like "cursor-desktop"
        let slug = normalized.replace(' ', "");
        if slug.len() >= 3 && !tokens.contains(&slug) {
            tokens.push(slug);
        }
        let slug_dash = normalized.replace(' ', "-");
        if slug_dash.len() >= 3 && !tokens.contains(&slug_dash) {
            tokens.push(slug_dash);
        }
    }
    // Deduplicate
    let mut seen = HashSet::new();
    tokens.retain(|t| seen.insert(t.clone()));
    tokens
}

fn path_matches_tokens(path_lower: &str, tokens: &[String], publisher_lower: &str) -> bool {
    for t in tokens {
        if path_lower.contains(t.as_str()) {
            return true;
        }
    }
    if !publisher_lower.is_empty() && publisher_lower.len() >= 3 && path_lower.contains(publisher_lower) {
        return true;
    }
    false
}

fn calc_artifact_size(path: &Path) -> Option<u64> {
    let md = std::fs::symlink_metadata(path).ok()?;
    if md.file_type().is_symlink() {
        return None;
    }
    if md.is_file() {
        return Some(md.len());
    }
    if md.is_dir() {
        // Use utility helper which caps at 10GB and skips symlinks
        return crate::utils::get_directory_size(path).ok();
    }
    None
}

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

        // Tokenized search: "Cursor Desktop" also matches "...\Cursor" and "...\cursor-desktop"
        let tokens = tokenize_app_name(&app.name);
        let publisher_lower = app
            .publisher
            .as_ref()
            .map(|p| p.to_lowercase())
            .unwrap_or_default();

        // Always include the install_location itself as a high-confidence artifact if it exists
        if let Some(ref loc) = app.install_location {
            if loc.exists() && !crate::utils::is_protected_path(loc, &greek_common::PROTECTED_PATHS.iter().map(|s| s.to_string()).collect::<Vec<_>>()) {
                let mut a = LeftoverArtifact::new(
                    if loc.is_dir() { ArtifactType::Directory } else { ArtifactType::File },
                    loc.clone(),
                );
                a.size_bytes = calc_artifact_size(loc);
                a.confidence = 0.95;
                a.description = format!("Install location for {}", app.name);
                a.safety_level = self.determine_safety_level(&a);
                artifacts.push(a);
            }
        }

        // If user configured no explicit roots, fall back to comprehensive scan (all drives + AppData)
        let scan_roots: Vec<PathBuf> = if self.scan_directories.is_empty() {
            build_comprehensive_scan_roots()
        } else {
            self.scan_directories.clone()
        };

        // Offload CPU-bound walk to blocking pool; collect per-root to avoid lock contention
        let tokens_clone = tokens.clone();
        let publisher_clone = publisher_lower.clone();
        let roots_clone = scan_roots.clone();
        let found_per_root = tokio::task::spawn_blocking(move || {
            let mut all: Vec<Vec<LeftoverArtifact>> = Vec::new();
            for dir in &roots_clone {
                if !dir.exists() { continue; }
                // Windows folder is huge — cap depth to 2 (e.g. C:\Windows\Cursor -> depth 2)
                let max_depth = if dir.to_string_lossy().to_lowercase().ends_with("\\windows") || dir.to_string_lossy().to_lowercase().ends_with("/windows") { 2 } else { 4 };
                let mut local = Vec::new();
                for entry in WalkDir::new(dir).max_depth(max_depth).follow_links(false).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();
                    // Skip reparse points / junctions already handled by !follow_links, but still skip symlink metadata
                    if std::fs::symlink_metadata(path).map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                        continue;
                    }
                    // Never flag documents/images as leftovers (user request: exclude .pdf, .doc, .ppt, .xlsx, .jpg, .png, etc.)
                    if path.is_file() && is_excluded_doc_image(path) {
                        continue;
                    }
                    let path_lower = path.to_string_lossy().to_lowercase();
                    if path_matches_tokens(&path_lower, &tokens_clone, &publisher_clone) {
                        let art_type = if path.is_dir() { ArtifactType::Directory } else { ArtifactType::File };
                        let mut art = LeftoverArtifact::new(art_type, path.to_path_buf());
                        art.description = format!("Potential leftover for {}", tokens_clone.join(", "));
                        art.size_bytes = calc_artifact_size(path);
                        local.push(art);
                    }
                }
                all.push(local);
            }
            all
        }).await.map_err(|e| greek_common::GreekError::AnalysisError(format!("spawn_blocking join: {}", e)))?;

        for mut v in found_per_root { artifacts.extend(v.drain(..)); }

        // Dedup: keep only top-most directory when a parent and child both match
        // e.g. C:\Users\rohit\AppData\Roaming\Cursor and ...\Cursor\User\Workspace -> keep parent only
        artifacts.sort_by(|a,b| a.path.to_string_lossy().len().cmp(&b.path.to_string_lossy().len()));
        let mut deduped: Vec<LeftoverArtifact> = Vec::new();
        for art in artifacts {
            let is_child_of_existing = deduped.iter().any(|p| art.path.starts_with(&p.path) && art.path != p.path);
            if !is_child_of_existing {
                deduped.push(art);
            }
        }
        artifacts = deduped;

        // Also check for install_location already added may duplicate; dedup by path
        {
            let mut seen = HashSet::new();
            artifacts.retain(|a| seen.insert(a.path.to_string_lossy().to_lowercase()));
        }

        // Score each artifact
        for artifact in &mut artifacts {
            artifact.confidence = self.score_confidence(artifact, app);
            artifact.safety_level = self.determine_safety_level(artifact);
            // Ensure size is populated (calc may have failed due to permission)
            if artifact.size_bytes.is_none() {
                artifact.size_bytes = calc_artifact_size(&artifact.path);
            }
        }

        // Sort by size descending (most impactful first) then path
        artifacts.sort_by(|a,b| b.size_bytes.cmp(&a.size_bytes).then_with(|| a.path.cmp(&b.path)));

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
        let tokens = tokenize_app_name(app_name);
        let publisher_lower = publisher.to_lowercase();
        let mut artifacts = Vec::new();
        let max_depth = if directory.to_string_lossy().to_lowercase().ends_with("\\windows") || directory.to_string_lossy().to_lowercase().ends_with("/windows") { 2 } else { 4 };
        for entry in WalkDir::new(directory)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if std::fs::symlink_metadata(path).map(|m| m.file_type().is_symlink()).unwrap_or(false) { continue; }
            if path.is_file() && is_excluded_doc_image(path) { continue; }
            let path_str = path.to_string_lossy().to_lowercase();
            let name_match = tokens.iter().any(|t| path_str.contains(t.as_str()));
            let pub_match = !publisher_lower.is_empty() && publisher_lower.len() >= 3 && path_str.contains(&publisher_lower);
            if name_match || pub_match {
                let artifact_type = if path.is_dir() { ArtifactType::Directory } else { ArtifactType::File };
                let mut artifact = LeftoverArtifact::new(artifact_type, path.to_path_buf());
                artifact.description = format!("Potential leftover for {}", app_name);
                artifact.size_bytes = calc_artifact_size(path);
                artifacts.push(artifact);
            }
        }
        // Dedup parent/child
        artifacts.sort_by(|a,b| a.path.to_string_lossy().len().cmp(&b.path.to_string_lossy().len()));
        let mut deduped = Vec::new();
        for art in artifacts { if !deduped.iter().any(|p: &LeftoverArtifact| art.path.starts_with(&p.path) && art.path != p.path) { deduped.push(art); } }
        Ok(deduped)
    }

    async fn scan_for_orphans(&self, _directory: &PathBuf) -> Result<Vec<LeftoverArtifact>> {
        // Orphan detection without an app context is inherently low-confidence.
        // Previously this flagged any directory older than 30 days (massive false positives).
        // Now it returns empty until cross-referenced against installed-apps list.
        // Callers should use analyze(&app) which is keyword-driven.
        Ok(Vec::new())
    }

    fn determine_safety_level(&self, artifact: &LeftoverArtifact) -> SafetyLevel {
        // V008: use the canonical PROTECTED_PATHS list for system path detection
        // instead of naive substring matching, which produces false positives
        // (e.g. C:UsersAliceAppDataLocalMicrosoftWindows...).
        let protected = greek_common::PROTECTED_PATHS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        if crate::utils::is_protected_path(&artifact.path, &protected) {
            return SafetyLevel::Critical;
        }

        let path_str = artifact.path.to_string_lossy().to_lowercase();

        // AppData is generally safe for user apps
        if path_str.contains("appdata") {
            return SafetyLevel::Safe;
        }

        // Default to caution
        SafetyLevel::Caution
    }
}

/// Registry leftover analyzer — scans HKLM/HKCU for keys/values containing app tokens
/// Covers whole device registry including Uninstall, Software, Run, Services, App Paths, etc.
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

#[cfg(target_os = "windows")]
impl RegistryLeftoverAnalyzer {
    pub fn new() -> Self {
        Self {
            base: BaseLeftoverAnalyzer::new("registry"),
        }
    }

    fn token_match(hay: &str, tokens: &[String]) -> bool {
        let lower = hay.to_lowercase();
        tokens.iter().any(|t| lower.contains(t.as_str()))
    }

    fn scan_hive_recursive(
        &self,
        hive: greek_common::RegistryHive,
        path: &str,
        tokens: &[String],
        depth: usize,
        max_depth: usize,
        artifacts: &mut Vec<LeftoverArtifact>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        if depth > max_depth { return; }
        let root = match hive {
            greek_common::RegistryHive::Hklm => winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE),
            greek_common::RegistryHive::Hkcu => winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER),
        };
        let key = match root.open_subkey(path) {
            Ok(k) => k,
            Err(_) => return,
        };
        let hive_prefix = hive.as_str();
        // Check this key itself
        if Self::token_match(path, tokens) {
            let full = format!("{}\\{}", hive_prefix, path);
            if seen.insert(full.to_lowercase()) {
                let mut a = LeftoverArtifact::new(ArtifactType::RegistryKey, PathBuf::from(full.clone()));
                a.description = format!("Registry key containing app token: {}", path);
                a.confidence = 0.85;
                a.safety_level = SafetyLevel::Caution;
                artifacts.push(a);
            }
        }
        // Enumerate values in this key for token match
        for val in key.enum_values().filter_map(|v| v.ok()) {
            let (vname, vval) = val;
            let vname_lower = vname.to_lowercase();
            let vdata_str = match vval.vtype {
                winreg::enums::RegType::REG_SZ | winreg::enums::RegType::REG_EXPAND_SZ => {
                    String::from_utf8_lossy(&vval.bytes).to_lowercase()
                }
                _ => String::new(),
            };
            if Self::token_match(&vname_lower, tokens) || (!vdata_str.is_empty() && Self::token_match(&vdata_str, tokens)) {
                let full = format!("{}\\{}\\ [{}]", hive_prefix, path, vname);
                if seen.insert(full.to_lowercase()) {
                    let mut a = LeftoverArtifact::new(ArtifactType::RegistryValue, PathBuf::from(full));
                    a.description = format!("Registry value for {}", path);
                    a.confidence = 0.75;
                    a.safety_level = SafetyLevel::Caution;
                    artifacts.push(a);
                }
            }
        }
        if depth == max_depth { return; }
        for sub in key.enum_keys().filter_map(|k| k.ok()) {
            let sub_path = if path.is_empty() { sub.clone() } else { format!("{}\\{}", path, sub) };
            self.scan_hive_recursive(hive, &sub_path, tokens, depth + 1, max_depth, artifacts, seen);
            // Safety cap: avoid explosion (e.g. Installer\Products has 1000s)
            if artifacts.len() > 500 { break; }
        }
    }
}

#[cfg(target_os = "windows")]
#[async_trait]
impl LeftoverAnalyzer for RegistryLeftoverAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        self.base.analyzer_id()
    }

    async fn analyze(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        let tokens = tokenize_app_name(&app.name);
        let publisher_lower = app.publisher.as_ref().map(|p| p.to_lowercase()).unwrap_or_default();
        let mut all_tokens = tokens.clone();
        if !publisher_lower.is_empty() && publisher_lower.len() >= 3 {
            // also match publisher tokens split
            for t in publisher_lower.split(|c: char| !c.is_alphanumeric()).filter(|s| s.len()>=3) {
                if !all_tokens.contains(&t.to_string()) { all_tokens.push(t.to_string()); }
            }
        }
        let tokens_clone = all_tokens;
        let install_loc = app.install_location.clone();
        let app_name = app.name.clone();
        let artifacts = tokio::task::spawn_blocking(move || {
            use winreg::enums::*;
            use winreg::RegKey;
            let mut arts: Vec<LeftoverArtifact> = Vec::new();
            let mut seen = std::collections::HashSet::new();

            // Bases to scan per hive with per-base max depth
            let bases: Vec<(&str, usize)> = vec![
                ("Software", 3),
                ("Software\\WOW6432Node", 3),
                ("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall", 2),
                ("Software\\Microsoft\\Windows\\CurrentVersion\\App Paths", 2),
                ("Software\\Microsoft\\Windows\\CurrentVersion\\Run", 1),
                ("Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce", 1),
                ("System\\CurrentControlSet\\Services", 1),
                ("Software\\Classes", 2),
            ];

            for (base, depth) in bases {
                // HKLM
                let mut local = Vec::new();
                let mut local_seen = std::collections::HashSet::new();
                let analyzer = RegistryLeftoverAnalyzer::new();
                analyzer.scan_hive_recursive(greek_common::RegistryHive::Hklm, base, &tokens_clone, 0, depth, &mut local, &mut local_seen);
                for a in local {
                    if seen.insert(a.path.to_string_lossy().to_lowercase()) { arts.push(a); }
                }
                if arts.len() > 800 { break; }
                // HKCU
                let mut local2 = Vec::new();
                let mut seen2 = std::collections::HashSet::new();
                let analyzer2 = RegistryLeftoverAnalyzer::new();
                analyzer2.scan_hive_recursive(greek_common::RegistryHive::Hkcu, base, &tokens_clone, 0, depth, &mut local2, &mut seen2);
                for a in local2 {
                    if seen.insert(a.path.to_string_lossy().to_lowercase()) { arts.push(a); }
                }
                if arts.len() > 800 { break; }
            }

            // Also scan raw install location in registry values (e.g. InstallLocation) - ensure high confidence
            if let Some(loc) = install_loc {
                let loc_str = loc.to_string_lossy().to_lowercase();
                for a in &mut arts {
                    if loc_str.contains(&a.path.to_string_lossy().to_lowercase()) {
                        a.confidence = 0.9;
                    }
                }
                // ensure install location key itself is present if still in registry
                let _ = app_name;
            }

            arts
        }).await.map_err(|e| greek_common::GreekError::AnalysisError(format!("registry scan join: {}", e)))?;

        // Deduplicate and score
        let mut deduped = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for mut a in artifacts {
            let key = a.path.to_string_lossy().to_lowercase();
            if seen.insert(key) {
                a.safety_level = if crate::utils::is_protected_registry_path(&a.path.to_string_lossy()) {
                    SafetyLevel::Critical
                } else {
                    SafetyLevel::Caution
                };
                deduped.push(a);
            }
        }
        // Sort by confidence desc
        deduped.sort_by(|a,b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        if deduped.len() > 300 {
            deduped.truncate(300);
        }
        Ok(deduped)
    }

    async fn analyze_system(&self) -> Result<Vec<LeftoverArtifact>> {
        Ok(Vec::new())
    }

    fn score_confidence(&self, artifact: &LeftoverArtifact, app: &InstalledApp) -> f32 {
        let lower = artifact.path.to_string_lossy().to_lowercase();
        let tokens = tokenize_app_name(&app.name);
        if tokens.iter().any(|t| lower.contains(t.as_str())) { 0.85 } else { 0.5 }
    }
}

/// Junk / Temp leftover analyzer — scans Windows Temp, Prefetch, user Temp, cache, logs across all drives
#[cfg(target_os = "windows")]
pub struct JunkLeftoverAnalyzer {
    base: BaseLeftoverAnalyzer,
}

#[cfg(target_os = "windows")]
impl Default for JunkLeftoverAnalyzer {
    fn default() -> Self { Self::new() }
}

#[cfg(target_os = "windows")]
impl JunkLeftoverAnalyzer {
    pub fn new() -> Self { Self { base: BaseLeftoverAnalyzer::new("junk") } }
}

#[cfg(target_os = "windows")]
#[async_trait]
impl LeftoverAnalyzer for JunkLeftoverAnalyzer {
    fn analyzer_id(&self) -> &'static str { self.base.analyzer_id() }
    async fn analyze(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        let tokens = tokenize_app_name(&app.name);
        let publisher_lower = app.publisher.as_ref().map(|p| p.to_lowercase()).unwrap_or_default();
        let arts = tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            let mut junk_roots: Vec<PathBuf> = Vec::new();
            // Standard temp locations
            for var in ["TEMP", "TMP"] {
                if let Ok(v) = std::env::var(var) { junk_roots.push(PathBuf::from(v)); }
            }
            junk_roots.push(PathBuf::from("C:\\Windows\\Temp"));
            junk_roots.push(PathBuf::from("C:\\Windows\\Prefetch"));
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                let local_path = PathBuf::from(&local);
                junk_roots.push(local_path.join("Temp"));
                junk_roots.push(local_path.join("Cache"));
                junk_roots.push(PathBuf::from(format!("{}\\cache", local)));
            }
            // Per-drive temp
            for drive in enumerate_drives() {
                let d = drive.trim_end_matches('\\');
                junk_roots.push(PathBuf::from(format!("{}\\Temp", d)));
                junk_roots.push(PathBuf::from(format!("{}\\Windows\\Temp", d)));
            }
            let mut seen = std::collections::HashSet::new();
            for root in junk_roots {
                if !root.exists() { continue; }
                for entry in WalkDir::new(&root).max_depth(3).follow_links(false).into_iter().filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if std::fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(false) { continue; }
                    if p.is_file() && is_excluded_doc_image(p) { continue; }
                    let lower = p.to_string_lossy().to_lowercase();
                    let matches = tokens.iter().any(|t| lower.contains(t.as_str())) || (!publisher_lower.is_empty() && lower.contains(&publisher_lower));
                    // Also consider generic junk when inside app-named parent already captured - we want app-specific junk only
                    if matches {
                        if !seen.insert(lower.clone()) { continue; }
                        let is_dir = p.is_dir();
                        let mut a = LeftoverArtifact::new(if is_dir { ArtifactType::TempFile } else { ArtifactType::TempFile }, p.to_path_buf());
                        // Alternative: use Directory for folders, TempFile for files
                        if is_dir { a.artifact_type = ArtifactType::Directory; }
                        a.description = format!("Junk/temp for {}", tokens.join(", "));
                        a.size_bytes = calc_artifact_size(p);
                        a.confidence = 0.7;
                        a.safety_level = SafetyLevel::Safe; // temp is safe
                        out.push(a);
                    }
                    if out.len() > 200 { break; }
                }
            }
            out
        }).await.map_err(|e| greek_common::GreekError::AnalysisError(format!("junk scan join: {}", e)))?;
        Ok(arts)
    }
    async fn analyze_system(&self) -> Result<Vec<LeftoverArtifact>> { Ok(Vec::new()) }
    fn score_confidence(&self, _artifact: &LeftoverArtifact, _app: &InstalledApp) -> f32 { 0.65 }
}

/// Service leftover analyzer
#[cfg(target_os = "windows")]
pub struct ServiceLeftoverAnalyzer { base: BaseLeftoverAnalyzer }
#[cfg(target_os = "windows")]
impl Default for ServiceLeftoverAnalyzer { fn default() -> Self { Self::new() } }
#[cfg(target_os = "windows")]
impl ServiceLeftoverAnalyzer { pub fn new() -> Self { Self { base: BaseLeftoverAnalyzer::new("service") } } }
#[cfg(target_os = "windows")]
#[async_trait]
impl LeftoverAnalyzer for ServiceLeftoverAnalyzer {
    fn analyzer_id(&self) -> &'static str { self.base.analyzer_id() }
    async fn analyze(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        let tokens = tokenize_app_name(&app.name);
        let arts = tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            // Query services via PowerShell synchronously
            let ps = r#"Get-CimInstance Win32_Service | Select-Object Name, DisplayName | ConvertTo-Json"#;
            let output = std::process::Command::new("powershell.exe")
                .args(["-NoProfile","-NonInteractive","-Command", ps])
                .output();
            if let Ok(outp) = output {
                if outp.status.success() {
                    let json_str = String::from_utf8_lossy(&outp.stdout);
                    let trimmed = json_str.trim();
                    if !trimmed.is_empty() && trimmed != "null" {
                        let values: Vec<serde_json::Value> = if trimmed.starts_with('[') {
                            serde_json::from_str(trimmed).unwrap_or_default()
                        } else {
                            serde_json::from_str::<serde_json::Value>(trimmed).map(|v| vec![v]).unwrap_or_default()
                        };
                        for v in values {
                            let name = v.get("Name").and_then(|x| x.as_str()).unwrap_or("");
                            let display = v.get("DisplayName").and_then(|x| x.as_str()).unwrap_or("");
                            let lower = format!("{} {}", name, display).to_lowercase();
                            if tokens.iter().any(|t| lower.contains(t.as_str())) {
                                let mut a = LeftoverArtifact::new(ArtifactType::Service, PathBuf::from(format!("Service\\{}", name)));
                                a.description = format!("Service: {} ({})", name, display);
                                a.confidence = 0.8;
                                a.safety_level = SafetyLevel::Caution;
                                out.push(a);
                            }
                        }
                    }
                }
            }
            out
        }).await.map_err(|e| greek_common::GreekError::AnalysisError(format!("service scan join: {}", e)))?;
        Ok(arts)
    }
    async fn analyze_system(&self) -> Result<Vec<LeftoverArtifact>> { Ok(Vec::new()) }
    fn score_confidence(&self, _a: &LeftoverArtifact, _app: &InstalledApp) -> f32 { 0.8 }
}

/// Scheduled task leftover analyzer
#[cfg(target_os = "windows")]
pub struct TaskLeftoverAnalyzer { base: BaseLeftoverAnalyzer }
#[cfg(target_os = "windows")]
impl Default for TaskLeftoverAnalyzer { fn default() -> Self { Self::new() } }
#[cfg(target_os = "windows")]
impl TaskLeftoverAnalyzer { pub fn new() -> Self { Self { base: BaseLeftoverAnalyzer::new("tasks") } } }
#[cfg(target_os = "windows")]
#[async_trait]
impl LeftoverAnalyzer for TaskLeftoverAnalyzer {
    fn analyzer_id(&self) -> &'static str { self.base.analyzer_id() }
    async fn analyze(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        let tokens = tokenize_app_name(&app.name);
        let arts = tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            let ps = r#"Get-ScheduledTask | Select-Object TaskName, TaskPath | ConvertTo-Json -Depth 3"#;
            let output = std::process::Command::new("powershell.exe")
                .args(["-NoProfile","-NonInteractive","-Command", ps])
                .output();
            if let Ok(outp) = output {
                if outp.status.success() {
                    let json_str = String::from_utf8_lossy(&outp.stdout);
                    let trimmed = json_str.trim();
                    if !trimmed.is_empty() && trimmed != "null" {
                        let values: Vec<serde_json::Value> = if trimmed.starts_with('[') {
                            serde_json::from_str(trimmed).unwrap_or_default()
                        } else {
                            serde_json::from_str::<serde_json::Value>(trimmed).map(|v| vec![v]).unwrap_or_default()
                        };
                        for v in values {
                            let name = v.get("TaskName").and_then(|x| x.as_str()).unwrap_or("");
                            let path = v.get("TaskPath").and_then(|x| x.as_str()).unwrap_or("\\");
                            let lower = format!("{} {}", name, path).to_lowercase();
                            if tokens.iter().any(|t| lower.contains(t.as_str())) {
                                let mut a = LeftoverArtifact::new(ArtifactType::ScheduledTask, PathBuf::from(format!("{}\\{}", path, name)));
                                a.description = format!("Scheduled task: {} ({})", name, path);
                                a.confidence = 0.75;
                                a.safety_level = SafetyLevel::Caution;
                                out.push(a);
                            }
                        }
                    }
                }
            }
            out
        }).await.map_err(|e| greek_common::GreekError::AnalysisError(format!("task scan join: {}", e)))?;
        Ok(arts)
    }
    async fn analyze_system(&self) -> Result<Vec<LeftoverArtifact>> { Ok(Vec::new()) }
    fn score_confidence(&self, _a: &LeftoverArtifact, _app: &InstalledApp) -> f32 { 0.75 }
}

/// Shortcut / Startup analyzer - scans Start Menu, Desktop, Startup folders for .lnk matching app
#[cfg(target_os = "windows")]
pub struct ShortcutLeftoverAnalyzer { base: BaseLeftoverAnalyzer }
#[cfg(target_os = "windows")]
impl Default for ShortcutLeftoverAnalyzer { fn default() -> Self { Self::new() } }
#[cfg(target_os = "windows")]
impl ShortcutLeftoverAnalyzer { pub fn new() -> Self { Self { base: BaseLeftoverAnalyzer::new("shortcut") } } }
#[cfg(target_os = "windows")]
#[async_trait]
impl LeftoverAnalyzer for ShortcutLeftoverAnalyzer {
    fn analyzer_id(&self) -> &'static str { self.base.analyzer_id() }
    async fn analyze(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        let tokens = tokenize_app_name(&app.name);
        let out = tokio::task::spawn_blocking(move || {
            let mut roots = Vec::new();
            for var in ["ProgramData", "APPDATA"] {
                if let Ok(v) = std::env::var(var) {
                    roots.push(PathBuf::from(v));
                }
            }
            // Common shortcut locations
            let mut scan_dirs = Vec::new();
            if let Ok(pd) = std::env::var("ProgramData") { scan_dirs.push(PathBuf::from(format!("{}\\Microsoft\\Windows\\Start Menu", pd))); }
            if let Ok(appdata) = std::env::var("APPDATA") { scan_dirs.push(PathBuf::from(format!("{}\\Microsoft\\Windows\\Start Menu", appdata))); }
            if let Ok(up) = std::env::var("USERPROFILE") {
                scan_dirs.push(PathBuf::from(format!("{}\\Desktop", up)));
                scan_dirs.push(PathBuf::from(format!("{}\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs", up)));
                scan_dirs.push(PathBuf::from(format!("{}\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup", up)));
            }
            // Per-user Desktops
            for drive in enumerate_drives() {
                let d = drive.trim_end_matches('\\');
                let users = PathBuf::from(format!("{}\\Users", d));
                if users.exists() {
                    if let Ok(entries) = std::fs::read_dir(&users) {
                        for ent in entries.filter_map(|e| e.ok()) {
                            let p = ent.path();
                            if p.is_dir() {
                                scan_dirs.push(p.join("Desktop"));
                                scan_dirs.push(p.join("AppData").join("Roaming").join("Microsoft").join("Windows").join("Start Menu").join("Programs"));
                            }
                        }
                    }
                }
            }
            let mut arts = Vec::new();
            for dir in scan_dirs {
                if !dir.exists() { continue; }
                for entry in WalkDir::new(&dir).max_depth(3).follow_links(false).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.extension().map(|e| e.to_string_lossy().to_lowercase() != "lnk").unwrap_or(true) { continue; }
                    let lower = path.to_string_lossy().to_lowercase();
                    if tokens.iter().any(|t| lower.contains(t.as_str())) {
                        let mut a = LeftoverArtifact::new(ArtifactType::Shortcut, path.to_path_buf());
                        a.description = format!("Shortcut for {}", tokens.join(", "));
                        a.confidence = 0.8;
                        a.safety_level = SafetyLevel::Safe;
                        if let Ok(md) = std::fs::symlink_metadata(path) { a.size_bytes = Some(md.len()); }
                        arts.push(a);
                    }
                    if arts.len() > 100 { break; }
                }
            }
            arts
        }).await.map_err(|e| greek_common::GreekError::AnalysisError(format!("shortcut scan join: {}", e)))?;
        Ok(out)
    }
    async fn analyze_system(&self) -> Result<Vec<LeftoverArtifact>> { Ok(Vec::new()) }
    fn score_confidence(&self, _a: &LeftoverArtifact, _app: &InstalledApp) -> f32 { 0.8 }
}

/// Duplicate installer / download analyzer — finds duplicate setup files across all drives
/// without touching the actual installed location. Safe to delete.
#[cfg(target_os = "windows")]
pub struct DuplicateDownloadAnalyzer { base: BaseLeftoverAnalyzer }
#[cfg(target_os = "windows")]
impl Default for DuplicateDownloadAnalyzer { fn default() -> Self { Self::new() } }
#[cfg(target_os = "windows")]
impl DuplicateDownloadAnalyzer {
    pub fn new() -> Self { Self { base: BaseLeftoverAnalyzer::new("duplicate") } }
    fn is_under_install(path: &Path, install: Option<&PathBuf>) -> bool {
        if let Some(inst) = install {
            let p = path.to_string_lossy().to_lowercase();
            let i = inst.to_string_lossy().to_lowercase();
            // Normalize separators
            let pn = p.replace('\\', "/");
            let inn = i.replace('\\', "/");
            if pn == inn || pn.starts_with(&(inn.clone() + "/")) { return true; }
        }
        false
    }
}
#[cfg(target_os = "windows")]
#[async_trait]
impl LeftoverAnalyzer for DuplicateDownloadAnalyzer {
    fn analyzer_id(&self) -> &'static str { self.base.analyzer_id() }
    async fn analyze(&self, app: &InstalledApp) -> Result<Vec<LeftoverArtifact>> {
        let tokens = tokenize_app_name(&app.name);
        let install_loc = app.install_location.clone();
        let arts = tokio::task::spawn_blocking(move || {
            let mut scan_dirs: Vec<PathBuf> = Vec::new();
            let mut seen_roots = std::collections::HashSet::new();
            let mut push = |p: PathBuf| {
                if p.exists() {
                    let k = p.to_string_lossy().to_lowercase();
                    if seen_roots.insert(k) { scan_dirs.push(p); }
                }
            };
            // Per-user Downloads / Desktop / Documents
            for drive in enumerate_drives() {
                let d = drive.trim_end_matches('\\');
                let users = PathBuf::from(format!("{}\\Users", d));
                if users.exists() {
                    if let Ok(entries) = std::fs::read_dir(&users) {
                        for ent in entries.filter_map(|e| e.ok()) {
                            let up = ent.path();
                            if !up.is_dir() { continue; }
                            let name = up.file_name().and_then(|n| n.to_str()).unwrap_or("");
                            if ["Public","Default","Default User","All Users"].contains(&name) { continue; }
                            push(up.join("Downloads"));
                            push(up.join("Desktop"));
                            push(up.join("Documents"));
                            push(up.join("Downloads").join("")); // ensure
                        }
                    }
                }
                // Drive-root download folders
                push(PathBuf::from(format!("{}\\Downloads", d)));
                push(PathBuf::from(format!("{}\\Download", d)));
                // Drive root itself shallow (for D:\CursorSetup.exe)
                push(PathBuf::from(format!("{}\\", d)));
            }
            // Env current user
            if let Ok(up) = std::env::var("USERPROFILE") {
                let up_path = PathBuf::from(&up);
                push(up_path.join("Downloads"));
                push(up_path.join("Desktop"));
                push(up_path.join("Documents"));
            }
            if let Ok(pd) = std::env::var("PUBLIC") { push(PathBuf::from(pd).join("Downloads")); }

            let installer_exts = ["exe","msi","zip","7z","rar","dmg","pkg","appimage","deb","rpm","msix","msixbundle","tar","gz","tgz","iso"];
            let mut out = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for dir in scan_dirs {
                if !dir.exists() { continue; }
                // For drive roots like D:\ we limit depth to 2 to avoid full drive walk
                let is_drive_root = dir.to_string_lossy().len() <= 3; // e.g. "D:\"
                let max_depth = if is_drive_root { 2 } else { 3 };
                for entry in WalkDir::new(&dir).max_depth(max_depth).follow_links(false).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if !path.is_file() { continue; }
                    if std::fs::symlink_metadata(path).map(|m| m.file_type().is_symlink()).unwrap_or(false) { continue; }
                    if is_excluded_doc_image(path) { continue; }
                    // Skip if under install location (protect installed app)
                    if Self::is_under_install(path, install_loc.as_ref()) { continue; }
                    // Check extension
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                    // Handle double ext like .tar.gz
                    let ext2 = if ext == "gz" {
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase().ends_with(".tar.gz").then(|| "tar.gz".to_string()).unwrap_or(ext.clone())
                    } else { ext.clone() };
                    let is_installer = installer_exts.iter().any(|e| ext == *e || ext2 == *e) || lower_contains_installer_hint(&path.to_string_lossy().to_lowercase());
                    if !is_installer { continue; }
                    let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                    // Token match on filename (not full path to avoid matching parent folder like Users)
                    if !tokens.iter().any(|t| fname.contains(t.as_str())) { continue; }
                    // Also require reasonable size (skip tiny 0B)
                    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                    if size < 1024 { continue; } // skip <1KB
                    let lower = path.to_string_lossy().to_lowercase();
                    if !seen.insert(lower.clone()) { continue; }
                    let mut a = LeftoverArtifact::new(ArtifactType::File, path.to_path_buf());
                    a.description = format!("Duplicate installer/download for {} — safe to delete (original installed at {})", tokens.join(", "), install_loc.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "unknown".to_string()));
                    a.size_bytes = Some(size);
                    a.confidence = 0.9;
                    a.safety_level = SafetyLevel::Safe;
                    out.push(a);
                    if out.len() > 200 { break; }
                }
            }
            out
        }).await.map_err(|e| greek_common::GreekError::AnalysisError(format!("duplicate scan join: {}", e)))?;
        Ok(arts)
    }
    async fn analyze_system(&self) -> Result<Vec<LeftoverArtifact>> { Ok(Vec::new()) }
    fn score_confidence(&self, _a: &LeftoverArtifact, _app: &InstalledApp) -> f32 { 0.9 }
}

#[cfg(target_os = "windows")]
fn lower_contains_installer_hint(lower: &str) -> bool {
    lower.contains("setup") || lower.contains("installer") || lower.contains("install")
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
