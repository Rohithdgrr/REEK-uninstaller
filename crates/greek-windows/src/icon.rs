// Icon extraction and app enrichment for installed applications.
//
// Real icons are extracted from each app's .exe (or DisplayIcon) using the
// Windows Shell API via a single batched PowerShell invocation, cached as PNG
// files under %APPDATA%\REEK\icons. The dominant color of each icon is
// computed so the TUI can render a per-app colored avatar.

use greek_common::InstalledApp;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Extracts and caches real icons for installed applications.
pub struct IconExtractor {
    cache_dir: PathBuf,
}

/// One extraction job: an exe path and its cache target.
struct IconJob {
    idx: usize,
    exe: String,
    cache: PathBuf,
}

impl IconExtractor {
    pub fn new() -> Self {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let cache_dir = PathBuf::from(base).join("REEK").join("icons");
        let _ = std::fs::create_dir_all(&cache_dir);
        Self { cache_dir }
    }

    /// Find a reasonable icon source path (.exe/.ico/.dll) for an app.
    pub fn find_exe_path(app: &InstalledApp) -> Option<PathBuf> {
        // 1. DisplayIcon registry value (handles "path,index" and quoted paths)
        if let Some(icon) = app.metadata.get("display_icon") {
            if let Some(p) = Self::parse_icon_path(icon) {
                return Some(p);
            }
        }
        // 2. Install location is itself an exe
        if let Some(loc) = &app.install_location {
            if loc
                .extension()
                .map(|e| e.eq_ignore_ascii_case("exe"))
                .unwrap_or(false)
                && loc.exists()
            {
                return Some(loc.clone());
            }
        }
        // 3. Exe path derived from the uninstall string
        if let Some(p) = app.metadata.get("exe_path") {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    fn parse_icon_path(s: &str) -> Option<PathBuf> {
        let clean = s.split(',').next().unwrap_or("").trim().trim_matches('"');
        if clean.is_empty() {
            return None;
        }
        let path = PathBuf::from(clean);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    fn cache_file(&self, exe: &Path) -> PathBuf {
        let mut h = DefaultHasher::new();
        exe.to_string_lossy().hash(&mut h);
        self.cache_dir.join(format!("{:016x}.png", h.finish()))
    }

    /// Extract icons for all apps lacking one, using a single PowerShell run.
    /// Returns the number of icons extracted.
    pub fn extract_icons(&self, apps: &mut [InstalledApp]) -> usize {
        let mut jobs: Vec<IconJob> = Vec::new();
        for (i, app) in apps.iter_mut().enumerate() {
            if app.icon_path.is_some() {
                continue;
            }
            let Some(exe) = Self::find_exe_path(app) else {
                continue;
            };
            let cache = self.cache_file(&exe);
            if cache.exists() {
                app.icon_path = Some(cache);
            } else {
                jobs.push(IconJob {
                    idx: i,
                    exe: exe.to_string_lossy().into_owned(),
                    cache,
                });
            }
        }

        if jobs.is_empty() {
            return 0;
        }

        let script = build_ps_script(&jobs);
        let _ = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output();

        let mut count = 0;
        for job in &jobs {
            if job.cache.exists() {
                apps[job.idx].icon_path = Some(job.cache.clone());
                count += 1;
            }
        }
        count
    }

    /// Compute the dominant (average) color of an icon PNG.
    pub fn dominant_color(path: &Path) -> Option<(u8, u8, u8)> {
        let img = image::open(path).ok()?;
        let small = img.thumbnail(8, 8).to_rgba8();
        let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
        for px in small.pixels() {
            if px[3] > 128 {
                r += px[0] as u64;
                g += px[1] as u64;
                b += px[2] as u64;
                n += 1;
            }
        }
        if n == 0 {
            return None;
        }
        Some(((r / n) as u8, (g / n) as u8, (b / n) as u8))
    }

    /// Downsample an icon PNG to an 8x8 RGBA pixel buffer (64 * 4 bytes).
    pub fn icon_rgba_8x8(path: &Path) -> Option<Vec<u8>> {
        let img = image::open(path).ok()?;
        Some(img.thumbnail_exact(8, 8).to_rgba8().into_raw())
    }

    /// Post-process scanned apps: extract real icons, compute dominant colors,
    /// and fill in missing sizes from the install directory.
    pub fn enrich_apps(&self, apps: &mut [InstalledApp]) {
        let extracted = self.extract_icons(apps);
        tracing::info!("Extracted {} app icons", extracted);

        for app in apps.iter_mut() {
            if let Some(icon) = &app.icon_path {
                if !app.metadata.contains_key("icon_color") {
                    if let Some(c) = Self::dominant_color(icon) {
                        app.metadata
                            .insert("icon_color".into(), format!("{},{},{}", c.0, c.1, c.2));
                    }
                }
                if !app.metadata.contains_key("icon_rgba") {
                    if let Some(raw) = Self::icon_rgba_8x8(icon) {
                        use base64::Engine;
                        app.metadata.insert(
                            "icon_rgba".into(),
                            base64::engine::general_purpose::STANDARD.encode(raw),
                        );
                    }
                }
            }
            if app.size_bytes.is_none() {
                if let Some(loc) = &app.install_location {
                    if loc.exists() {
                        match dir_size(loc) {
                            Ok(size) if size > 0 => app.size_bytes = Some(size),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

impl Default for IconExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively compute a directory size (iterative, avoids stack overflow).
fn dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let rd = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for entry in rd.flatten() {
            let md = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.is_dir() {
                stack.push(entry.path());
            } else {
                total += md.len();
            }
        }
    }
    Ok(total)
}

fn ps_escape(s: &str) -> String {
    s.replace('\'', "''")
}

fn build_ps_script(jobs: &[IconJob]) -> String {
    let pairs: Vec<String> = jobs
        .iter()
        .map(|j| {
            format!(
                "@('{}','{}')",
                ps_escape(&j.exe),
                ps_escape(&j.cache.to_string_lossy())
            )
        })
        .collect();
    let mut sb = String::from("Add-Type -AssemblyName System.Drawing\r\n");
    sb.push_str(&format!("$jobs = @({})\r\n", pairs.join(",\r\n")));
    sb.push_str(
        "foreach ($j in $jobs) { try { $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($j[0]); if ($icon) { $b = $icon.ToBitmap(); $b.Save($j[1], [System.Drawing.Imaging.ImageFormat]::Png); $b.Dispose(); $icon.Dispose() } } catch {} }\r\nWrite-Output 'DONE'\r\n",
    );
    sb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_file_unique() {
        let ex = IconExtractor::new();
        let a = ex.cache_file(Path::new(r"C:\a\b.exe"));
        let b = ex.cache_file(Path::new(r"C:\a\c.exe"));
        assert_ne!(a, b);
    }

    #[test]
    fn test_parse_icon_path() {
        assert!(IconExtractor::parse_icon_path(r"C:\nonexistent\file.exe,0").is_none());
        assert!(IconExtractor::parse_icon_path("").is_none());
    }

    #[test]
    fn test_build_ps_script_no_trailing_comma() {
        let jobs = vec![
            IconJob {
                idx: 0,
                exe: r"C:\a\b.exe".into(),
                cache: PathBuf::from(r"C:\cache\1.png"),
            },
            IconJob {
                idx: 1,
                exe: r"C:\a\c.exe".into(),
                cache: PathBuf::from(r"C:\cache\2.png"),
            },
        ];
        let script = build_ps_script(&jobs);
        assert!(script.contains("@('C:\\a\\b.exe','C:\\cache\\1.png')"));
        assert!(script.contains("@('C:\\a\\c.exe','C:\\cache\\2.png')"));
        assert!(!script.contains(",)\r\n"));
        assert!(!script.contains("@('C:\\a\\c.exe','C:\\cache\\2.png'),"));
    }

    #[test]
    fn test_ps_escape() {
        assert_eq!(ps_escape("it's"), "it''s");
    }
}
