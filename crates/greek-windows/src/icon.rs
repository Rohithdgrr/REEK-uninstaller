// Icon extraction and app enrichment for installed applications.
//
// Icon sources that are plain images (Store logo PNGs, DisplayIcon .ico/.png)
// are decoded directly with the `image` crate. Executable sources get their
// icon extracted via the Windows Shell API using batched PowerShell
// invocations (chunked to stay under the Windows command-line length limit).
// Everything is cached as PNG under %APPDATA%\REEK\icons; the dominant color
// of each icon is computed so the TUI can render a per-app colored avatar.

use greek_common::InstalledApp;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Keep each PowerShell `-Command` script comfortably below the ~32k char
/// CreateProcess command-line limit (long scripts silently fail otherwise).
const PS_SCRIPT_BUDGET: usize = 24_000;
/// Fixed overhead (bytes) of the generated PowerShell script boilerplate.
/// High-quality extraction embeds a C# helper (~2.8k chars) so budget must account for it.
const PS_SCRIPT_OVERHEAD: usize = 3500;
/// Kill a hung powershell.exe instead of blocking the scan forever.
const PS_TIMEOUT: Duration = Duration::from_secs(45);
/// A cached PNG smaller than this is treated as truncated/corrupt.
const MIN_PNG_BYTES: u64 = 64;
/// Image extensions the `image` crate decodes directly (no shell round-trip).
const DIRECT_IMAGE_EXTS: [&str; 5] = ["png", "ico", "jpg", "jpeg", "bmp"];

/// Extracts and caches real icons for installed applications.
pub struct IconExtractor {
    cache_dir: PathBuf,
}

/// One extraction job: an icon source path and its cache target.
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

    /// Find a reasonable icon source path (.exe/.ico/.dll/.png) for an app.
    /// Covers DisplayIcon, exe_path, install_location directory search (2 levels deep),
    /// and .ico fallback so that every installed app gets a real icon.
    pub fn find_exe_path(app: &InstalledApp) -> Option<PathBuf> {
        // 1. DisplayIcon registry value (handles "path,index" and quoted paths) - most reliable.
        if let Some(icon) = app.metadata.get("display_icon") {
            if let Some(p) = Self::parse_icon_path(icon) {
                return Some(p);
            }
            // Try expanding env vars if raw path didn't exist (e.g. %ProgramFiles%\..)
            let expanded = expand_env_vars(icon);
            if let Some(p) = Self::parse_icon_path(&expanded) {
                return Some(p);
            }
        }
        // 2. Install location is itself an exe file.
        if let Some(loc) = &app.install_location {
            if loc
                .extension()
                .map(|e| e.eq_ignore_ascii_case("exe"))
                .unwrap_or(false)
                && loc.exists()
            {
                return Some(loc.clone());
            }
            // 2b. Install location is a directory: search for best exe inside (2 levels deep).
            // This covers Chrome (chrome.exe), Brave (brave.exe), Docker (Docker Desktop.exe), etc.
            if loc.is_dir() {
                if let Some(best) = Self::find_best_exe_in_dir(&app.name, loc) {
                    return Some(best);
                }
                // Fallback: look for any .ico directly (Store logos, etc.)
                if let Some(ico) = Self::find_best_ico_in_dir(loc) {
                    return Some(ico);
                }
            }
        }
        // 3. Exe path derived from the uninstall string. Generic installer
        // binaries (MsiExec, unins*, setup*) carry no useful icon - filtered.
        if let Some(p) = app.metadata.get("exe_path") {
            let path = PathBuf::from(p);
            if path.exists() && !poor_icon_source(&path) {
                return Some(path);
            }
        }
        // 4. Last resort: accept even poor icon sources (unins*.exe) rather than no icon.
        if let Some(p) = app.metadata.get("exe_path") {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
        }
        // 5. Last resort: scan install_location dir without poor filter (if we skipped earlier due to poor)
        if let Some(loc) = &app.install_location {
            if loc.is_dir() {
                if let Some(any_exe) = Self::find_any_exe_in_dir(loc) {
                    return Some(any_exe);
                }
                // Final fallback: use the directory itself (yields generic folder icon rather than initials)
                return Some(loc.clone());
            }
        }
        None
    }

    /// Search a directory up to 2 levels deep for the best .exe to use as icon.
    /// Prefers exes whose name contains the app name, then largest file.
    fn find_best_exe_in_dir(app_name: &str, dir: &Path) -> Option<PathBuf> {
        let candidates = Self::collect_exes(dir, 2, true);
        if candidates.is_empty() {
            return None;
        }
        Self::pick_best_exe(app_name, &candidates)
    }
    fn find_any_exe_in_dir(dir: &Path) -> Option<PathBuf> {
        let candidates = Self::collect_exes(dir, 2, false);
        if candidates.is_empty() {
            return None;
        }
        // Any exe at all, pick largest
        candidates.into_iter().max_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
    }
    fn find_best_ico_in_dir(dir: &Path) -> Option<PathBuf> {
        let mut stack: Vec<(PathBuf, usize)> = vec![(dir.to_path_buf(), 0)];
        let mut best: Option<PathBuf> = None;
        while let Some((d, depth)) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for entry in rd.flatten() {
                let Ok(ft) = entry.file_type() else { continue };
                let path = entry.path();
                if ft.is_dir() {
                    if depth < 2 {
                        stack.push((path, depth + 1));
                    }
                } else if ft.is_file() {
                    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("ico") || e.eq_ignore_ascii_case("png")) {
                        // Prefer .ico over .png for shell extraction
                        if best.is_none() || path.extension().is_some_and(|e| e.eq_ignore_ascii_case("ico")) {
                            best = Some(path.clone());
                            if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("ico")) {
                                // keep searching but ico is best; still allow
                            }
                        }
                    }
                }
            }
        }
        best
    }
    fn collect_exes(dir: &Path, max_depth: usize, filter_poor: bool) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let mut stack: Vec<(PathBuf, usize)> = vec![(dir.to_path_buf(), 0)];
        while let Some((d, depth)) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for entry in rd.flatten() {
                let Ok(ft) = entry.file_type() else { continue };
                let path = entry.path();
                if ft.is_dir() {
                    if depth < max_depth {
                        stack.push((path, depth + 1));
                    }
                } else if ft.is_file() {
                    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")) {
                        if filter_poor && poor_icon_source(&path) {
                            continue;
                        }
                        out.push(path);
                    }
                }
            }
        }
        out
    }
    fn pick_best_exe(app_name: &str, candidates: &[PathBuf]) -> Option<PathBuf> {
        let norm_app = app_name.to_lowercase().replace(' ', "").replace('-', "");
        // Score: 0 = stem contains app or app contains stem, 1 = otherwise. Lower is better, then larger file wins.
        let mut scored: Vec<(u8, u64, PathBuf)> = candidates
            .iter()
            .map(|p| {
                let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
                let norm_stem = stem.replace(' ', "").replace('-', "").replace('_', "");
                let score = if norm_stem.contains(&norm_app) || norm_app.contains(&norm_stem) { 0u8 } else { 1u8 };
                let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                (score, size, p.clone())
            })
            .collect();
        scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
        scored.into_iter().next().map(|(_, _, p)| p)
    }

    /// Parse a DisplayIcon-style value into an existing path. Registry values
    /// look like `C:\path\app.exe`, `"C:\path\app.exe",0` or
    /// `"C:\path\app.ico",-3`. Only a *trailing* numeric index is stripped so
    /// paths containing commas survive.
    fn parse_icon_path(s: &str) -> Option<PathBuf> {
        let t = s.trim();
        let stripped = match t.rfind(',') {
            Some(pos) if is_index_suffix(&t[pos + 1..]) => t[..pos].trim(),
            _ => t,
        };
        let clean = stripped.trim_matches('"').trim();
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
        // _hq suffix forces re-extraction at 256px after the high-quality upgrade;
        // old 32px caches (without suffix) are ignored so icons become crisp.
        self.cache_dir.join(format!("{:016x}_hq.png", h.finish()))
    }

    /// A cache entry is only trusted when it exists and has a plausible size
    /// for a PNG (protects against 0-byte files from crashed runs).
    fn valid_cache(path: &Path) -> bool {
        std::fs::metadata(path)
            .map(|m| m.len() >= MIN_PNG_BYTES)
            .unwrap_or(false)
    }

    fn direct_image_source(path: &Path) -> bool {
        path.extension().is_some_and(|e| {
            let e = e.to_string_lossy().to_ascii_lowercase();
            DIRECT_IMAGE_EXTS.contains(&e.as_str())
        })
    }

    /// Extract icons for all apps lacking one.
    /// Returns the number of icons available afterwards.
    pub fn extract_icons(&self, apps: &mut [InstalledApp]) -> usize {
        let mut shell_jobs: Vec<IconJob> = Vec::new();
        let mut direct_jobs: Vec<IconJob> = Vec::new();

        for (i, app) in apps.iter_mut().enumerate() {
            if app.icon_path.is_some() {
                continue;
            }
            let Some(source) = Self::find_exe_path(app) else {
                continue;
            };
            let cache = self.cache_file(&source);

            if Self::valid_cache(&cache) {
                app.icon_path = Some(cache);
                continue;
            }
            // Corrupt/truncated leftover from an earlier crashed run: drop it
            // so extraction below can retry.
            if cache.exists() {
                let _ = std::fs::remove_file(&cache);
            }

            let job = IconJob {
                idx: i,
                exe: source.to_string_lossy().into_owned(),
                cache,
            };
            if Self::direct_image_source(Path::new(&job.exe)) {
                direct_jobs.push(job);
            } else {
                shell_jobs.push(job);
            }
        }

        // Plain-image sources (Store logo PNGs, DisplayIcon .ico/.png): decode
        // directly with the image crate - no PowerShell needed.
        for job in &direct_jobs {
            match image::open(Path::new(&job.exe)) {
                Ok(img) => {
                    let saved = std::fs::File::create(&job.cache).and_then(|mut f| {
                        img.write_to(&mut f, image::ImageFormat::Png)
                            .map_err(std::io::Error::other)
                    });
                    if let Err(e) = saved {
                        tracing::warn!("Failed to convert icon {}: {}", job.exe, e);
                        let _ = std::fs::remove_file(&job.cache);
                    }
                }
                Err(e) => tracing::warn!("Failed to decode icon {}: {}", job.exe, e),
            }
        }

        // Executable sources need the Windows Shell API: run batched
        // PowerShell jobs, chunked to respect the command-line limit.
        for chunk in chunk_shell_jobs(&shell_jobs, PS_SCRIPT_BUDGET) {
            let script = build_ps_script(&chunk);
            if let Err(e) = run_ps_script(&script) {
                tracing::warn!("Icon extraction batch ({} jobs) failed: {}", chunk.len(), e);
            }
        }

        let mut count = 0;
        for job in direct_jobs.iter().chain(shell_jobs.iter()) {
            if Self::valid_cache(&job.cache) {
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

    /// Decode once and derive both the dominant-color string and the
    /// base64-encoded 8x8 RGBA buffer used by the TUI.
    fn icon_assets(path: &Path) -> Option<(String, String)> {
        use base64::Engine;
        let img = image::open(path).ok()?;

        let color_img = img.thumbnail(8, 8).to_rgba8();
        let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
        for px in color_img.pixels() {
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
        let color = format!("{},{},{}", (r / n) as u8, (g / n) as u8, (b / n) as u8);

        let rgba_b64 = base64::engine::general_purpose::STANDARD
            .encode(img.thumbnail_exact(8, 8).to_rgba8().into_raw());
        Some((color, rgba_b64))
    }

    /// Post-process scanned apps: extract real icons, compute dominant colors,
    /// and fill in missing sizes from the install directory.
    pub fn enrich_apps(&self, apps: &mut [InstalledApp]) {
        let extracted = self.extract_icons(apps);
        tracing::info!("Extracted {} app icons", extracted);

        for app in apps.iter_mut() {
            if let Some(icon) = &app.icon_path {
                match Self::icon_assets(icon) {
                    Some((color, rgba)) => {
                        app.metadata.entry("icon_color".into()).or_insert(color);
                        app.metadata.entry("icon_rgba".into()).or_insert(rgba);
                    }
                    None => {
                        // Undecodable cached PNG: remove it so the next scan
                        // retries extraction instead of failing forever.
                        tracing::warn!("Removing unreadable cached icon: {}", icon.display());
                        let _ = std::fs::remove_file(icon);
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

fn expand_env_vars(s: &str) -> String {
    // Expand %VAR% segments using environment variables (e.g. %ProgramFiles%).
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let mut var = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == '%' {
                    chars.next();
                    break;
                }
                var.push(nc);
                chars.next();
            }
            if !var.is_empty() {
                if let Ok(val) = std::env::var(&var) {
                    out.push_str(&val);
                    continue;
                }
                // also try upper case
                if let Ok(val) = std::env::var(var.to_uppercase()) {
                    out.push_str(&val);
                    continue;
                }
            }
            out.push('%');
            out.push_str(&var);
            out.push('%');
        } else {
            out.push(c);
        }
    }
    out
}

/// Heuristic: executables that never carry the app's real icon because they
/// are generic installer/bootstrap binaries.
pub(crate) fn poor_icon_source(path: &Path) -> bool {
    let Some(stem) = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
    else {
        return false;
    };
    if stem == "msiexec" {
        return true;
    }
    const BAD_PREFIXES: [&str; 6] = ["unins", "unvise", "setup", "update", "patch", "install"];
    BAD_PREFIXES.iter().any(|p| stem.starts_with(p))
}

fn is_index_suffix(s: &str) -> bool {
    let s = s.trim();
    let digits = s.strip_prefix('-').unwrap_or(s);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
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

/// Split shell jobs into chunks whose generated script stays within `budget`
/// characters, avoiding the ~32k CreateProcess command-line limit.
fn chunk_shell_jobs(jobs: &[IconJob], budget: usize) -> Vec<Vec<&IconJob>> {
    let mut chunks: Vec<Vec<&IconJob>> = Vec::new();
    let mut current: Vec<&IconJob> = Vec::new();
    let mut used = PS_SCRIPT_OVERHEAD;

    for job in jobs {
        let cost = job.exe.len() + job.cache.to_string_lossy().len() + 12;
        if !current.is_empty() && used + cost > budget {
            chunks.push(std::mem::take(&mut current));
            used = PS_SCRIPT_OVERHEAD;
        }
        used += cost;
        current.push(job);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// Run one PowerShell script with a timeout; returns stderr tail on failure.
fn run_ps_script(script: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};

    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn powershell.exe: {e}"))?;

    let deadline = Instant::now() + PS_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                }
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                let tail: String = stderr
                    .chars()
                    .skip(stderr.chars().count().saturating_sub(300))
                    .collect();
                return Err(format!("powershell {}: {}", status, tail.trim()));
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timed out after {}s", PS_TIMEOUT.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn build_ps_script(chunk: &[&IconJob]) -> String {
    let pairs: Vec<String> = chunk
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
    // High-quality path: try SHGetImageList Jumbo (256x256) -> ExtraLarge (48x48) before falling back to ExtractAssociatedIcon (32x32).
    // Guard against re-definition when multiple batches run in same PowerShell process (would error "type already exists").
    sb.push_str(r#"if (-not ([System.Management.Automation.PSTypeName]'HiResIcon').Type) { Add-Type @"
using System;
using System.Drawing;
using System.Runtime.InteropServices;
public class HiResIcon {
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Auto)] public struct SHFILEINFO { public IntPtr hIcon; public int iIcon; public uint dwAttributes; [MarshalAs(UnmanagedType.ByValTStr, SizeConst=260)] public string szDisplayName; [MarshalAs(UnmanagedType.ByValTStr, SizeConst=80)] public string szTypeName; }
  [DllImport("shell32.dll", CharSet=CharSet.Auto)] public static extern IntPtr SHGetFileInfo(string pszPath,uint dwFileAttributes, ref SHFILEINFO psfi,uint cbSizeFileInfo,uint uFlags);
  [DllImport("shell32.dll")] public static extern int SHGetImageList(int iImageList, ref Guid riid, ref IntPtr ppv);
  [ComImport, Guid("46EB5926-582E-4017-9FDF-E8998DAA0950"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)] public interface IImageList { [PreserveSig] int GetIcon(int i,int flags, ref IntPtr pIcon); }
  const uint SHGFI_SYSICONINDEX=0x4000;
  const int SHIL_JUMBO=4, SHIL_EXTRALARGE=2, ILD_TRANSPARENT=1;
  public static Icon GetJumbo(string path){
    SHFILEINFO sfi=new SHFILEINFO();
    SHGetFileInfo(path,0,ref sfi,(uint)Marshal.SizeOf(typeof(SHFILEINFO)),SHGFI_SYSICONINDEX);
    Guid iid=new Guid("46EB5926-582E-4017-9FDF-E8998DAA0950");
    IntPtr iml=IntPtr.Zero;
    if(SHGetImageList(SHIL_JUMBO, ref iid, ref iml)==0){
      IImageList list=(IImageList)Marshal.GetObjectForIUnknown(iml);
      IntPtr hIcon=IntPtr.Zero; list.GetIcon(sfi.iIcon, ILD_TRANSPARENT, ref hIcon);
      if(hIcon!=IntPtr.Zero) return Icon.FromHandle(hIcon);
    }
    if(SHGetImageList(SHIL_EXTRALARGE, ref iid, ref iml)==0){
      IImageList list=(IImageList)Marshal.GetObjectForIUnknown(iml);
      IntPtr hIcon=IntPtr.Zero; list.GetIcon(sfi.iIcon, ILD_TRANSPARENT, ref hIcon);
      if(hIcon!=IntPtr.Zero) return Icon.FromHandle(hIcon);
    }
    return null;
  }
}
"@
}
"#,
    );
    sb.push_str(&format!("$jobs = @({})\r\n", pairs.join(",\r\n")));
    sb.push_str(
        "foreach ($j in $jobs) { try { $ico=[HiResIcon]::GetJumbo($j[0]); if(-not $ico){ $ico=[System.Drawing.Icon]::ExtractAssociatedIcon($j[0]) } if($ico){ $bmp=$ico.ToBitmap(); $bmp.Save($j[1],[System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose(); $ico.Dispose() } } catch {} }\r\nWrite-Output 'DONE'\r\n",
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
    fn test_parse_icon_path_strips_trailing_index_only() {
        let dir = std::env::temp_dir().join(format!("reek_parse_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("my,app.png");
        std::fs::write(&file, b"x").unwrap();
        let p = file.to_string_lossy().into_owned();

        // Trailing numeric index suffixes (quoted or not) are stripped.
        assert_eq!(
            IconExtractor::parse_icon_path(&format!("\"{p}\",0")),
            Some(file.clone())
        );
        assert_eq!(
            IconExtractor::parse_icon_path(&format!("{p},-3")),
            Some(file.clone())
        );
        // A comma inside the path without a trailing index survives intact.
        assert_eq!(
            IconExtractor::parse_icon_path(&format!("\"{p}\"")),
            Some(file.clone())
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_is_index_suffix() {
        assert!(is_index_suffix("0"));
        assert!(is_index_suffix("-3"));
        assert!(is_index_suffix(" 12 "));
        assert!(!is_index_suffix(""));
        assert!(!is_index_suffix("-"));
        assert!(!is_index_suffix("app.png\""));
    }

    #[test]
    fn test_poor_icon_source() {
        assert!(poor_icon_source(Path::new(
            r"C:\Program Files\App\unins000.exe"
        )));
        assert!(poor_icon_source(Path::new(
            r"C:\Windows\System32\msiexec.exe"
        )));
        assert!(poor_icon_source(Path::new(r"C:\App\SETUP.EXE")));
        assert!(poor_icon_source(Path::new(r"C:\App\updater.exe")));
        assert!(!poor_icon_source(Path::new(
            r"C:\Program Files\7-Zip\7zFM.exe"
        )));
        assert!(!poor_icon_source(Path::new(r"C:\App\firefox.exe")));
    }

    #[test]
    fn test_direct_image_source() {
        assert!(IconExtractor::direct_image_source(Path::new(
            r"C:\a\Logo.png"
        )));
        assert!(IconExtractor::direct_image_source(Path::new(
            r"C:\a\APP.ICO"
        )));
        assert!(!IconExtractor::direct_image_source(Path::new(
            r"C:\a\app.exe"
        )));
        assert!(!IconExtractor::direct_image_source(Path::new(r"C:\a\app")));
    }

    #[test]
    fn test_dominant_color_png_roundtrip() {
        let dir = std::env::temp_dir().join(format!("reek_icon_rt_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("red.png");
        let img = image::RgbaImage::from_pixel(32, 32, image::Rgba([200u8, 30, 40, 255]));
        img.save_with_format(&png, image::ImageFormat::Png).unwrap();

        assert_eq!(IconExtractor::dominant_color(&png), Some((200, 30, 40)));
        assert_eq!(
            IconExtractor::icon_rgba_8x8(&png).map(|v| v.len()),
            Some(256)
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_chunk_shell_jobs_respects_budget() {
        let long_exe = format!(r"C:\\{}", "x".repeat(200));
        let jobs: Vec<IconJob> = (0..10)
            .map(|i| IconJob {
                idx: i,
                exe: long_exe.clone(),
                cache: PathBuf::from(r"C:\cache\out.png"),
            })
            .collect();

        let chunks = chunk_shell_jobs(&jobs, 600);
        // Each job costs ~230 chars; a 600-char budget cannot hold many.
        assert!(chunks.len() >= 4);
        // No job may be lost or duplicated by chunking.
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, 10);
        // Chunks preserve original order.
        let flat: Vec<usize> = chunks.iter().flatten().map(|j| j.idx).collect();
        assert_eq!(flat, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn test_build_ps_script_no_trailing_comma() {
        let jobs = [
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
        let refs: Vec<&IconJob> = jobs.iter().collect();
        let script = build_ps_script(&refs);
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
