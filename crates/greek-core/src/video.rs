// Video scanner — finds all video files across whole device for Movies section
use greek_common::{Result, VideoEntry};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

const VIDEO_EXTS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp", "3gpp", "mts",
    "m2ts", "ts", "vob", "ogv", "asf", "rm", "rmvb", "divx", "f4v", "amv", "mpe", "mp2", "m2v",
    "svi", "mxf", "roq", "nsv", "nuv", "drc", "gifv", "qt", "yuv", "m4s", "mk3d", "hevc", "av1",
    "mpd", "wmx", "dav",
];

fn is_video(path: &Path) -> bool {
    // Use lossy conversion so non-UTF8 filenames are still detected.
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    VIDEO_EXTS.contains(&ext.as_str())
}

/// Protected paths for the *read-only* video scan.
///
/// `PROTECTED_PATHS` contains `C:\Users`, which is correct for deletions
/// but wrong for a read-only scan — every Videos/Downloads/Desktop root
/// lives under it, so the old check skipped all C: results. Filter it out
/// here. Deletion keeps its own (narrower) guard in `delete_videos`.
fn scan_protected_paths() -> Vec<String> {
    greek_common::PROTECTED_PATHS
        .iter()
        .filter(|s| !s.eq_ignore_ascii_case(r"C:\Users"))
        .map(|s| s.to_string())
        .collect()
}

/// Narrower guard for deletions: allow video files under user profiles,
/// still block OS / app install locations.
fn delete_protected_paths() -> Vec<String> {
    greek_common::PROTECTED_PATHS
        .iter()
        .filter(|s| !s.eq_ignore_ascii_case(r"C:\Users"))
        .map(|s| s.to_string())
        .collect()
}

fn drive_label(p: &Path) -> String {
    let s = p.to_string_lossy();
    s.chars()
        .take(2)
        .collect::<String>()
        .to_uppercase()
        .replace(":", "")
}

pub struct VideoScanner {
    min_size_bytes: u64,
    max_depth: usize,
}

impl Default for VideoScanner {
    fn default() -> Self {
        Self {
            min_size_bytes: 1024 * 1024,
            max_depth: 8,
        }
    }
}

impl VideoScanner {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_options(min_size_bytes: u64, max_depth: usize) -> Self {
        Self {
            min_size_bytes,
            max_depth,
        }
    }

    fn push_if_exists(
        roots: &mut Vec<PathBuf>,
        seen: &mut std::collections::HashSet<String>,
        p: PathBuf,
    ) {
        if p.exists() {
            let k = p.to_string_lossy().to_lowercase();
            if seen.insert(k) {
                roots.push(p);
            }
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
                    if !up.is_dir() {
                        continue;
                    }
                    let uname = up
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if ["Public", "Default", "Default User", "All Users"]
                        .iter()
                        .any(|s| s.eq_ignore_ascii_case(&uname))
                    {
                        continue;
                    }
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Videos"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Downloads"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Desktop"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("Documents"));
                    Self::push_if_exists(&mut roots, &mut seen, up.join("OneDrive"));
                }
            }
        }
        // All drives: for each drive, add known media folders if they exist.
        // NOTE: do NOT add the bare `Users` folder — dedupe_roots() keeps the
        // shortest root and would collapse every per-user Videos/Downloads
        // folder into one giant `C:\Users` walk. Per-user enumeration above
        // already adds the precise subfolders.
        for letter in b'A'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            if !Path::new(&drive).exists() {
                continue;
            }
            let dp = PathBuf::from(&drive);
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Videos"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Movies"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Films"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Media"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Plex"));
            Self::push_if_exists(&mut roots, &mut seen, dp.join("Downloads"));
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
        let roots = crate::utils::dedupe_roots(Self::build_roots());
        let min_size = self.min_size_bytes;
        let max_depth = self.max_depth;
        let entries = tokio::task::spawn_blocking(move || {
            use rayon::prelude::*;
            // Read-only scan: C:\Users is allowed (see scan_protected_paths).
            let protected: Vec<String> = scan_protected_paths();
            // Walk each root on the thread pool; merge at the end.
            let mut all: Vec<VideoEntry> = roots
                .into_par_iter()
                .map(|root| {
                    let mut local = Vec::new();
                    let is_drive_root = root.to_string_lossy().len() <= 3;
                    let depth = if is_drive_root { 8 } else { max_depth.max(8) };
                    for entry_result in WalkDir::new(&root)
                        .max_depth(depth)
                        .follow_links(true)
                        .into_iter()
                    {
                        let entry = match entry_result {
                            Ok(e) => e,
                            Err(err) => {
                                tracing::debug!("video walk skip entry: {}", err);
                                continue;
                            }
                        };
                        // Extension check first (no syscalls); reuse the dir
                        // entry metadata instead of a second stat call.
                        if !is_video(entry.path()) {
                            continue;
                        }
                        let meta = match entry.metadata() {
                            Ok(m) => m,
                            Err(err) => {
                                tracing::debug!(
                                    "video walk skip metadata {}: {}",
                                    entry.path().display(),
                                    err
                                );
                                continue;
                            }
                        };
                        if !meta.is_file() {
                            continue;
                        }
                        let size = meta.len();
                        if size < min_size {
                            continue;
                        } // skip thumbs
                        let path = entry.path();
                        if greek_common::is_protected_path(path, &protected) {
                            continue;
                        }
                        let modified = meta
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                            .map(|dt| dt.naive_utc());
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let ext = path
                            .extension()
                            .map(|e| e.to_string_lossy().to_lowercase())
                            .unwrap_or_default();
                        let drive = drive_label(path);
                        local.push(VideoEntry {
                            id: Uuid::new_v4(),
                            path: path.to_path_buf(),
                            name,
                            extension: ext,
                            size_bytes: size,
                            size_display: humansize::format_size(size, humansize::BINARY),
                            modified,
                            drive,
                        });
                        if local.len() > 5000 {
                            break;
                        } // cap per root
                    }
                    local
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .collect();
            // Dedup by path, biggest first, hard cap.
            let mut seen = std::collections::HashSet::new();
            let mut dedup = Vec::new();
            for v in all.drain(..) {
                let k = v.path.to_string_lossy().to_lowercase();
                if seen.insert(k) {
                    dedup.push(v);
                }
            }
            dedup.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));
            if dedup.len() > 5000 {
                dedup.truncate(5000);
            }
            dedup
        })
        .await
        .map_err(|e| greek_common::GreekError::ScanError(format!("video scan join: {}", e)))?;
        Ok(entries)
    }

    /// Delete selected videos (move to recycle or direct)
    pub async fn delete_videos(&self, paths: Vec<PathBuf>) -> Result<Vec<String>> {
        let mut deleted = Vec::new();
        let mut errors = Vec::new();
        // Allow video files under user profiles; still block OS locations.
        let protected = delete_protected_paths();
        for p in paths {
            // Safety: only video files may be deleted through this path.
            if !is_video(&p) {
                errors.push(format!("Blocked non-video: {}", p.display()));
                continue;
            }
            if greek_common::is_protected_path(&p, &protected) {
                errors.push(format!("Blocked protected: {}", p.display()));
                continue;
            }
            // Recycle bin (not permanent delete) — matches the UI promise.
            // NOTE: must NOT use delete_file() here: it enforces the full
            // PROTECTED_PATHS list including C:\Users, so every video under
            // a user profile would be refused. The video-appropriate guards
            // (is_video + delete_protected_paths) already ran above.
            let res = tokio::task::spawn_blocking({
                let pp = p.clone();
                move || crate::utils::move_to_recycle_bin(&pp)
            })
            .await;
            match res {
                Ok(Ok(_)) => deleted.push(p.to_string_lossy().to_string()),
                Ok(Err(e)) => errors.push(format!("{}: {}", p.display(), e)),
                Err(e) => errors.push(format!("join {}: {}", p.display(), e)),
            }
        }
        if !errors.is_empty() && deleted.is_empty() {
            return Err(greek_common::GreekError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                errors.join("; "),
            )));
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
        assert!(is_video(Path::new("clip.m4s")));
        assert!(is_video(Path::new("clip.hevc")));
        assert!(!is_video(Path::new("doc.pdf")));
    }

    #[test]
    fn test_user_profile_allowed_in_scan() {
        // C:\Users must NOT be treated as protected for the read-only scan,
        // otherwise no C: videos are ever found.
        let protected = scan_protected_paths();
        assert!(!greek_common::is_protected_path(
            Path::new(r"C:\Users\alice\Videos\movie.mp4"),
            &protected
        ));
        // OS locations must still be blocked.
        assert!(greek_common::is_protected_path(
            Path::new(r"C:\Windows\System32\movie.mp4"),
            &protected
        ));
    }

    #[test]
    fn test_user_profile_allowed_in_delete_guards() {
        // Delete path must accept video files under user profiles (recycle
        // bin), still block OS locations and non-videos.
        let protected = delete_protected_paths();
        let user_video = Path::new(r"C:\Users\alice\Videos\movie.mp4");
        assert!(is_video(user_video));
        assert!(!greek_common::is_protected_path(user_video, &protected));
        assert!(!is_video(Path::new(r"C:\Users\alice\doc.pdf")));
        assert!(greek_common::is_protected_path(
            Path::new(r"C:\Windows\System32\movie.mp4"),
            &protected
        ));
    }
}
