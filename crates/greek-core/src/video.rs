// Video scanner — finds all video files across whole device for Movies section
use greek_common::{Result, VideoEntry};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use uuid::Uuid;

const VIDEO_EXTS: &[&str] = &[
    "mp4","mkv","avi","mov","wmv","flv","webm","m4v","mpg","mpeg","3gp","3gpp",
    "mts","m2ts","ts","vob","ogv","asf","rm","rmvb","divx","f4v","amv","mpe",
    "mp2","m2v","svi","mxf","roq","nsv","nuv","drc","gifv","qt","yuv",
];

fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn drive_label(p: &Path) -> String {
    let s = p.to_string_lossy();
    s.chars().take(2).collect::<String>().to_uppercase().replace(":", "")
}

pub struct VideoScanner {
    min_size_bytes: u64,
    max_depth: usize,
}

impl Default for VideoScanner {
    fn default() -> Self {
        Self { min_size_bytes: 1024 * 1024, max_depth: 6 }
    }
}

impl VideoScanner {
    pub fn new() -> Self { Self::default() }
    pub fn with_options(min_size_bytes: u64, max_depth: usize) -> Self {
        Self { min_size_bytes, max_depth }
    }

    fn push_if_exists(roots: &mut Vec<PathBuf>, seen: &mut std::collections::HashSet<String>, p: PathBuf) {
        if p.exists() {
            let k = p.to_string_lossy().to_lowercase();
            if seen.insert(k) { roots.push(p); }
        }
    }
    /// Build roots: all drives + known video locations
    fn build_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        let mut seen = std::collections::HashSet::new();
        // Env dirs
        for var in ["USERPROFILE", "PUBLIC"] {
            if let Ok(v) = std::env::var(var) {
                let base = PathBuf::from(v);
                Self::push_if_exists(&mut roots, &mut seen, base.join("Videos"));
                Self::push_if_exists(&mut roots, &mut seen, base.join("Downloads"));
                Self::push_if_exists(&mut roots, &mut seen, base.join("Desktop"));
                Self::push_if_exists(&mut roots, &mut seen, base.join("Documents"));
            }
        }
        // Per-user enumeration
        let users_root = PathBuf::from("C:\\Users");
        if users_root.exists() {
            if let Ok(entries) = std::fs::read_dir(&users_root) {
                for ent in entries.filter_map(|e| e.ok()) {
                    let up = ent.path();
                    if !up.is_dir() { continue; }
                    let name = up.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if ["Public","Default","Default User","All Users"].contains(&name) { continue; }
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Videos"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Downloads"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Desktop"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Documents"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("OneDrive"));
                }
            }
        }
        // All drives: for each drive, add Videos, Movies, Downloads if exists
        for letter in b'A'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            if !Path::new(&drive).exists() { continue; }
            let dp = PathBuf::from(&drive);
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Users"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Videos"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Movies"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Films"));
            if drive != "C:\\" && dp.exists() {
                Self::push_if_exists(&mut roots, &mut seen, dp.clone());
            }
        }
        if roots.is_empty() {
            Self::push_if_exists(&mut roots, &mut seen, PathBuf::from("C:\\Users"));
        }
        roots
    }

    pub async fn scan_all(&self) -> Result<Vec<VideoEntry>> {
        let roots = Self::build_roots();
        let min_size = self.min_size_bytes;
        let max_depth = self.max_depth;
        let entries = tokio::task::spawn_blocking(move || {
            let mut all = Vec::new();
            for root in roots {
                let is_drive_root = root.to_string_lossy().len() <= 3;
                let depth = if is_drive_root { 4 } else { max_depth };
                for entry in WalkDir::new(&root)
                    .max_depth(depth)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let path = entry.path();
                    if !path.is_file() { continue; }
                    if !is_video(path) { continue; }
                    // Skip tiny files < min_size (avoid thumbs)
                    let meta = match std::fs::metadata(path) { Ok(m) => m, Err(_) => continue };
                    let size = meta.len();
                    if size < min_size { continue; }
                    // Skip protected paths
                    let protected = greek_common::PROTECTED_PATHS.iter().map(|s| s.to_string()).collect::<Vec<_>>();
                    if greek_common::is_protected_path(path, &protected) { continue; }
                    let modified = meta.modified().ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .and_then(|d| chrono::NaiveDateTime::from_timestamp_opt(d.as_secs() as i64, 0));
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                    let drive = path.to_string_lossy().chars().take(2).collect::<String>().to_uppercase().replace(":", "");
                    all.push(VideoEntry {
                        id: Uuid::new_v4(),
                        path: path.to_path_buf(),
                        name,
                        extension: ext,
                        size_bytes: size,
                        size_display: humansize::format_size(size, humansize::BINARY),
                        modified,
                        drive,
                    });
                    if all.len() > 5000 { break; } // cap
                }
            }
            // Dedup by path
            let mut seen = std::collections::HashSet::new();
            let mut dedup = Vec::new();
            for v in all {
                let k = v.path.to_string_lossy().to_lowercase();
                if seen.insert(k) { dedup.push(v); }
            }
            dedup.sort_by(|a,b| b.size_bytes.cmp(&a.size_bytes));
            dedup
        }).await.map_err(|e| greek_common::GreekError::ScanError(format!("video scan join: {}", e)))?;
        Ok(entries)
    }

    /// Delete selected videos (move to recycle or direct)
    pub async fn delete_videos(&self, paths: Vec<PathBuf>) -> Result<Vec<String>> {
        let mut deleted = Vec::new();
        let mut errors = Vec::new();
        for p in paths {
            // Safety: protected check
            let protected = greek_common::PROTECTED_PATHS.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            if greek_common::is_protected_path(&p, &protected) {
                errors.push(format!("Blocked protected: {}", p.display()));
                continue;
            }
            // Try recycle bin first
            let res = tokio::task::spawn_blocking({
                let pp = p.clone();
                move || crate::utils::delete_file(&pp)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_is_video() {
        assert!(is_video(Path::new("movie.mp4")));
        assert!(is_video(Path::new("clip.MKV")));
        assert!(!is_video(Path::new("doc.pdf")));
    }
}
