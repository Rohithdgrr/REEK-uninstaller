// Dev modules scanner — finds node_modules, venv, target, dist etc. across whole device
use greek_common::{DevModuleEntry, DevModuleKind, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct Pattern {
    folder: &'static str,
    kind: DevModuleKind,
    exact: bool, // true = folder name must equal, false = contains
}

const PATTERNS: &[Pattern] = &[
    Pattern { folder: "node_modules", kind: DevModuleKind::NodeModules, exact: true },
    Pattern { folder: ".venv", kind: DevModuleKind::PythonVenv, exact: true },
    Pattern { folder: "venv", kind: DevModuleKind::PythonVenv, exact: true },
    Pattern { folder: "env", kind: DevModuleKind::PythonVenv, exact: true },
    Pattern { folder: ".env", kind: DevModuleKind::PythonVenv, exact: true },
    Pattern { folder: "__pycache__", kind: DevModuleKind::PythonCache, exact: true },
    Pattern { folder: ".pytest_cache", kind: DevModuleKind::PythonCache, exact: true },
    Pattern { folder: ".mypy_cache", kind: DevModuleKind::PythonCache, exact: true },
    Pattern { folder: ".tox", kind: DevModuleKind::PythonCache, exact: true },
    Pattern { folder: "target", kind: DevModuleKind::RustTarget, exact: true },
    Pattern { folder: "dist", kind: DevModuleKind::Dist, exact: true },
    Pattern { folder: "build", kind: DevModuleKind::Build, exact: true },
    Pattern { folder: "out", kind: DevModuleKind::Build, exact: true },
    Pattern { folder: ".next", kind: DevModuleKind::NextBuild, exact: true },
    Pattern { folder: ".nuxt", kind: DevModuleKind::NextBuild, exact: true },
    Pattern { folder: ".output", kind: DevModuleKind::NextBuild, exact: true },
    Pattern { folder: ".svelte-kit", kind: DevModuleKind::NextBuild, exact: true },
    Pattern { folder: ".astro", kind: DevModuleKind::NextBuild, exact: true },
    Pattern { folder: "vendor", kind: DevModuleKind::Vendor, exact: true },
    Pattern { folder: ".gradle", kind: DevModuleKind::GradleCache, exact: true },
    Pattern { folder: ".parcel-cache", kind: DevModuleKind::Build, exact: true },
];

// Heuristic to avoid scanning huge protected/irrelevant trees
fn should_skip_dir(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_lowercase();
    // Skip protected and heavy system dirs
    for p in greek_common::PROTECTED_PATHS {
        if lower.starts_with(&p.to_lowercase().replace('\\', "/")) && lower != p.to_lowercase().replace('\\', "/") {
            // Allow scanning Users but skip Windows
            if lower.contains("c:/windows") { return true; }
        }
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    // Skip .git, .hg, .svn
    if [".git",".hg",".svn",".idea",".vscode"].contains(&name.as_str()) { return true; }
    false
}

pub struct DevModulesScanner {
    max_depth: usize,
    min_size_bytes: u64,
}

impl Default for DevModulesScanner {
    fn default() -> Self { Self { max_depth: 6, min_size_bytes: 1024 } }
}

impl DevModulesScanner {
    pub fn new() -> Self { Self::default() }

    fn push_if_exists(roots: &mut Vec<PathBuf>, seen: &mut std::collections::HashSet<String>, p: PathBuf) {
        if p.exists() {
            let k = p.to_string_lossy().to_lowercase();
            if seen.insert(k) { roots.push(p); }
        }
    }
    fn build_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for var in ["USERPROFILE", "HOME"] {
            if let Ok(v) = std::env::var(var) {
                let base = PathBuf::from(v);
                Self::push_if_exists(&mut roots, &mut seen, base.join("Documents"));
                Self::push_if_exists(&mut roots, &mut seen, base.join("Desktop"));
                Self::push_if_exists(&mut roots, &mut seen, base.clone());
                Self::push_if_exists(&mut roots, &mut seen, base.join("Projects"));
                Self::push_if_exists(&mut roots, &mut seen, base.join("code"));
                Self::push_if_exists(&mut roots, &mut seen, base.join("dev"));
                Self::push_if_exists(&mut roots, &mut seen, base.join("workspace"));
                Self::push_if_exists(&mut roots, &mut seen, base.join("repos"));
                Self::push_if_exists(&mut roots, &mut seen, base.join("src"));
            }
        }
        let users_root = PathBuf::from("C:\\Users");
        if users_root.exists() {
            if let Ok(entries) = std::fs::read_dir(&users_root) {
                for ent in entries.filter_map(|e| e.ok()) {
                    let up = ent.path();
                    if !up.is_dir() { continue; }
                    let name = up.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if ["Public","Default","Default User","All Users"].contains(&name) { continue; }
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Documents"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Desktop"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Projects"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("code"));
                    Self::push_if_exists(&mut roots, &mut seen, up.clone());
                }
            }
        }
        for letter in b'A'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            if !Path::new(&drive).exists() { continue; }
            let dp = PathBuf::from(&drive);
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Projects"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("code"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("dev"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("workspace"));
            if drive != "C:\\" {
                Self::push_if_exists(&mut roots, &mut seen, dp.clone());
            }
        }
        if roots.is_empty() { Self::push_if_exists(&mut roots, &mut seen, PathBuf::from("C:\\Users")); }
        roots
    }

    pub async fn scan_all(&self) -> Result<Vec<DevModuleEntry>> {
        let roots = Self::build_roots();
        let max_depth = self.max_depth;
        let entries = tokio::task::spawn_blocking(move || {
            let mut all: Vec<DevModuleEntry> = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for root in roots {
                walk_scan(&root, max_depth, &mut all, &mut seen);
                if all.len() > 2000 { break; }
            }
            // Dedup and sort by size desc
            let mut dedup: Vec<DevModuleEntry> = Vec::new();
            let mut seen_path = std::collections::HashSet::new();
            for e in all {
                let k = e.path.to_string_lossy().to_lowercase();
                if seen_path.insert(k) { dedup.push(e); }
            }
            dedup.sort_by(|a,b| b.size_bytes.cmp(&a.size_bytes));
            if dedup.len() > 1000 { dedup.truncate(1000); }
            dedup
        }).await.map_err(|e| greek_common::GreekError::ScanError(format!("dev scan join: {}", e)))?;
        Ok(entries)
    }

    pub async fn delete_modules(&self, paths: Vec<PathBuf>) -> Result<Vec<String>> {
        let mut deleted = Vec::new();
        let mut errors = Vec::new();
        for p in paths {
            let protected = greek_common::PROTECTED_PATHS.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            if greek_common::is_protected_path(&p, &protected) {
                errors.push(format!("Blocked protected: {}", p.display()));
                continue;
            }
            // Ensure it's a known dev pattern (safety)
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            let is_known = PATTERNS.iter().any(|pt| if pt.exact { name == pt.folder } else { name.contains(pt.folder) });
            if !is_known {
                errors.push(format!("Not a known dev module: {}", p.display()));
                continue;
            }
            let res = tokio::task::spawn_blocking({
                let pp = p.clone();
                move || crate::utils::delete_directory(&pp)
            }).await;
            match res {
                Ok(Ok(_)) => deleted.push(p.to_string_lossy().to_string()),
                Ok(Err(e)) => errors.push(format!("{}: {}", p.display(), e)),
                Err(e) => errors.push(format!("join {}: {}", p.display(), e)),
            }
        }
        if !errors.is_empty() && deleted.is_empty() {
            return Err(greek_common::GreekError::IoError(std::io::Error::new(std::io::ErrorKind::Other, errors.join("; "))));
        }
        Ok(deleted)
    }

    pub async fn delete_all(&self, entries: Vec<DevModuleEntry>) -> Result<Vec<String>> {
        let paths: Vec<PathBuf> = entries.into_iter().map(|e| e.path).collect();
        self.delete_modules(paths).await
    }
}

fn walk_scan(root: &Path, max_depth: usize, out: &mut Vec<DevModuleEntry>, seen: &mut std::collections::HashSet<String>) {
    let mut stack = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > max_depth { continue; }
        if should_skip_dir(&dir) { continue; }
        let entries = match std::fs::read_dir(&dir) { Ok(e) => e, Err(_) => continue };
        for ent in entries.filter_map(|e| e.ok()) {
            let path = ent.path();
            let is_dir = path.is_dir();
            if !is_dir { continue; }
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            // Check if this folder matches any dev pattern
            for pat in PATTERNS {
                let matches = if pat.exact { fname == pat.folder } else { fname.contains(pat.folder) };
                if matches {
                    let lower = path.to_string_lossy().to_lowercase();
                    if seen.contains(&lower) { continue; }
                    seen.insert(lower.clone());
                    // Don't descend into this matched dir (avoid scanning inside node_modules deeply for nested node_modules)
                    // But still calculate size
                    let (size, count) = calc_dir_stats(&path);
                    let drive = path.to_string_lossy().chars().take(2).collect::<String>().to_uppercase().replace(":", "");
                    out.push(DevModuleEntry {
                        id: Uuid::new_v4(),
                        path: path.clone(),
                        name: path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
                        kind: pat.kind,
                        language: pat.kind.language().to_string(),
                        size_bytes: size,
                        size_display: humansize::format_size(size, humansize::BINARY),
                        file_count: count,
                        drive,
                    });
                    // Don't push children if it's node_modules (too many)
                    if pat.folder == "node_modules" || pat.folder == "target" || pat.folder == ".venv" {
                        // skip descending
                    } else {
                        // For other small caches, still don't descend deeply
                    }
                    break;
                }
            }
            // Continue walk if not matched or if we want to find nested modules (e.g. monorepo)
            // Push to stack to scan deeper
            if depth + 1 <= max_depth {
                // Avoid descending into huge node_modules/target we already reported — skip
                let fname2 = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                let is_matched = PATTERNS.iter().any(|p| if p.exact { fname2 == p.folder } else { fname2.contains(p.folder) });
                if !is_matched {
                    stack.push((path, depth + 1));
                } else {
                    // For target/node_modules, still allow finding nested modules inside parent projects? e.g. project/target
                    // We already reported this target, but there could be other modules at deeper levels beyond this folder's parent?
                    // No need to descend inside target/node_modules
                }
            }
        }
    }
}

fn calc_dir_stats(path: &Path) -> (u64, usize) {
    let mut size = 0u64;
    let mut count = 0usize;
    for entry in WalkDir::new(path).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(m) = entry.metadata() {
                size = size.saturating_add(m.len());
                count += 1;
                if size > greek_common::MAX_TOTAL_SCAN_SIZE_BYTES { break; }
            }
        }
        if count > 500_000 { break; } // cap
    }
    (size, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_patterns() {
        assert!(PATTERNS.iter().any(|p| p.folder == "node_modules"));
    }
}
