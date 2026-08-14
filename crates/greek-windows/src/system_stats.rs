// System statistics collection for the TUI status bar.
//
// CPU / RAM / swap / disks / processes come from `sysinfo`; battery comes
// from WMI via the `wmi` crate (direct COM); GPU utilization + VRAM come from
// the `GPU Engine` / `GPU Adapter Memory` performance counters, queried with a
// single short PowerShell `Get-Counter` call (the WMI GPU perf classes are
// missing on many systems).

use std::collections::HashMap;
use std::time::Duration;
use serde::Deserialize;
use wmi::{COMLibrary, WMIConnection};

/// Field names must match WMI property names exactly.
#[derive(Debug, Deserialize)]
struct VideoControllerRow {
    Name: Option<String>,
    AdapterRAM: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct BatteryRow {
    EstimatedChargeRemaining: Option<u64>,
    BatteryStatus: Option<u64>,
}

/// Live resource usage of a single running process. Keyed by lowercase exe path.
#[derive(Debug, Clone, Default)]
pub struct ProcessUsage {
    pub pid: u32,
    pub name: String,
    pub exe_path: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub virtual_memory: u64,
    pub run_time_secs: u64,
    pub started_at: Option<u64>,
    pub threads: usize,
    pub read_bytes: u64,
    pub written_bytes: u64,
    pub gpu_usage_pct: f32,
    pub vram_bytes: u64,
}

/// Aggregated GPU state plus per-process GPU utilization / VRAM.
#[derive(Debug, Clone, Default)]
pub struct GpuInfo {
    pub usage_pct: f32,
    pub vram_used_bytes: u64,
    pub vram_total_bytes: u64,
    pub per_pid: HashMap<u32, (f32, u64)>,
}

#[derive(Debug, Clone, Default)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub disks: Vec<DiskStat>,
    pub gpu: Option<GpuStat>,
    pub battery: Option<BatteryStat>,
    pub uptime_secs: u64,
    pub process_count: usize,
    /// Per-process resource usage keyed by lowercase exe path.
    pub processes: HashMap<String, ProcessUsage>,
}

#[derive(Debug, Clone)]
pub struct DiskStat {
    pub label: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

impl DiskStat {
    pub fn usage_pct(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes as f32 / self.total_bytes as f32) * 100.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuStat {
    pub name: String,
    pub usage_pct: f32,
    pub vram_used_bytes: u64,
    pub vram_total_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct BatteryStat {
    pub percent: u8,
    pub charging: bool,
}

/// Collects system stats. Must be created and used on one thread because it
/// owns a COM library handle for WMI.
pub struct SystemStatsCollector {
    sys: sysinfo::System,
    disks: sysinfo::Disks,
    wmi: Option<WMIConnection>,
    gpu_name: Option<String>,
    gpu_vram_total: Option<u64>,
}

impl SystemStatsCollector {
    pub fn new() -> Self {
        let sys = sysinfo::System::new();
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let wmi = Self::connect_wmi();
        let (gpu_name, gpu_vram_total) = wmi
            .as_ref()
            .and_then(Self::gpu_static_info)
            .unwrap_or((None, None));
        Self {
            sys,
            disks,
            wmi,
            gpu_name,
            gpu_vram_total,
        }
    }

    pub fn collect(&mut self) -> SystemStats {
        // Give the CPU counters a moment so the delta between the initial
        // refresh and this one is meaningful (avoids a bogus 100% first sample).
        std::thread::sleep(Duration::from_millis(250));
        self.sys
            .refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All);
        self.disks.refresh();

        let ram_total = self.sys.total_memory();
        let ram_used = self.sys.used_memory();
        let swap_total = self.sys.total_swap();
        let swap_used = self.sys.used_swap();

        let disks = self
            .disks
            .iter()
            .filter(|d| {
                d.total_space() > 0
                    && d.file_system().to_str().map_or(false, |s| !s.is_empty())
            })
            .map(|d| DiskStat {
                label: d.mount_point().to_string_lossy().trim_end_matches('\\').to_string(),
                used_bytes: d.total_space() - d.available_space(),
                total_bytes: d.total_space(),
            })
            .collect();

        let gpu_info = Self::gpu_info();
        let gpu_per_pid = gpu_info.as_ref().map(|g| g.per_pid.clone()).unwrap_or_default();
        let gpu = gpu_info.map(|g| GpuStat {
            name: self.gpu_name.clone().unwrap_or_default(),
            usage_pct: g.usage_pct,
            vram_used_bytes: g.vram_used_bytes,
            vram_total_bytes: if g.vram_total_bytes > 0 {
                g.vram_total_bytes
            } else {
                self.gpu_vram_total.unwrap_or(0)
            },
        });

        let mut processes = HashMap::new();
        for (_, p) in self.sys.processes() {
            let Some(exe) = p.exe() else { continue };
            let exe_lower = exe.to_string_lossy().to_lowercase();
            let pid = p.pid().as_u32();
            let (gpu_usage_pct, vram_bytes) = gpu_per_pid.get(&pid).copied().unwrap_or((0.0, 0));
            let du = p.disk_usage();
            processes.insert(
                exe_lower.clone(),
                ProcessUsage {
                    pid,
                    name: p.name().to_string_lossy().into_owned(),
                    exe_path: exe_lower,
                    cpu_usage: p.cpu_usage(),
                    memory_bytes: p.memory(),
                    virtual_memory: p.virtual_memory(),
                    run_time_secs: p.run_time(),
                    started_at: Some(p.start_time()),
                    threads: p.tasks().map(|t| t.len()).unwrap_or(0),
                    read_bytes: du.total_read_bytes,
                    written_bytes: du.total_written_bytes,
                    gpu_usage_pct,
                    vram_bytes,
                },
            );
        }

        let battery = self.wmi.as_ref().and_then(Self::battery_info);

        SystemStats {
            cpu_usage: self.sys.global_cpu_usage(),
            ram_used_bytes: ram_used,
            ram_total_bytes: ram_total,
            swap_used_bytes: swap_used,
            swap_total_bytes: swap_total,
            disks,
            gpu,
            battery,
            uptime_secs: sysinfo::System::uptime(),
            process_count: self.sys.processes().len(),
            processes,
        }
    }

    fn connect_wmi() -> Option<WMIConnection> {
        let com = COMLibrary::new().ok()?;
        WMIConnection::new(com).ok()
    }

    /// Static GPU info (name) fetched once via WMI.
    fn gpu_static_info(w: &WMIConnection) -> Option<(Option<String>, Option<u64>)> {
        let rows = w
            .raw_query::<VideoControllerRow>("SELECT Name, AdapterRAM FROM Win32_VideoController")
            .ok()?;
        let row = rows.into_iter().next()?;
        Some((row.Name, row.AdapterRAM))
    }

    /// GPU info via performance counters: aggregated 3D engine utilization,
    /// dedicated VRAM used / limit, plus per-process utilization and VRAM.
    /// One PowerShell `Get-Counter` call. Returns None when the counters are
    /// unavailable.
    fn gpu_info() -> Option<GpuInfo> {
        const SCRIPT: &str = r#"
$s = Get-Counter @('\GPU Engine(*)\Utilization Percentage','\GPU Adapter Memory(*)\Dedicated Usage','\GPU Adapter Memory(*)\Dedicated Limit','\GPU Process Memory(*)\Dedicated Usage') -ErrorAction SilentlyContinue
if (-not $s) { -1; -1; -1; exit }
$eng = $s.CounterSamples | Where-Object { $_.Path -like '*Utilization*' -and $_.InstanceName -like '*engtype_3d' }
$use = $s.CounterSamples | Where-Object { $_.Path -like '*Adapter Memory*' -and $_.Path -like '*Dedicated Usage*' } | Measure-Object -Property CookedValue -Sum
$lim = $s.CounterSamples | Where-Object { $_.Path -like '*Dedicated Limit*' } | Select-Object -First 1
if ($null -eq $eng -or @($eng).Count -eq 0) { 0 } else { [math]::Round(($eng | Measure-Object -Property CookedValue -Sum).Sum, 1) }
if ($null -eq $use.Sum) { 0 } else { $use.Sum }
if ($null -ne $lim) { $lim.CookedValue } else { -1 }
$per = @{}
foreach ($smp in $eng) {
  if ($smp.InstanceName -match '^pid_(\d+)') {
    $k = $Matches[1]
    if (-not $per.ContainsKey($k)) { $per[$k] = @{ u = 0.0; v = 0.0 } }
    $per[$k].u += [double]$smp.CookedValue
  }
}
foreach ($smp in $s.CounterSamples) {
  if ($smp.Path -like '*Process Memory*' -and $smp.InstanceName -match '^pid_(\d+)') {
    $k = $Matches[1]
    if (-not $per.ContainsKey($k)) { $per[$k] = @{ u = 0.0; v = 0.0 } }
    $per[$k].v += [double]$smp.CookedValue
  }
}
$per.GetEnumerator() | Sort-Object { [int]$_.Key } | ForEach-Object { '{0}|{1}|{2}' -f $_.Key, [math]::Round($_.Value.u, 2), $_.Value.v }
"#;

        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().map(str::trim).filter(|l| !l.is_empty()).collect();

        let mut nums = Vec::with_capacity(3);
        let mut rest = 0usize;
        for l in &lines {
            if nums.len() >= 3 {
                break;
            }
            if let Ok(v) = l.parse::<f64>() {
                nums.push(v);
                rest += 1;
            }
        }
        if nums.len() < 3 || nums[0] < 0.0 || nums[1] < 0.0 {
            return None;
        }
        let (usage, used) = (nums[0].min(100.0) as f32, nums[1] as u64);
        let limit = if nums[2] < 0.0 { 0 } else { nums[2] as u64 };

        let mut per_pid = HashMap::new();
        for l in &lines[rest..] {
            let mut parts = l.split('|');
            let (Some(pid_s), Some(u_s), Some(v_s)) = (parts.next(), parts.next(), parts.next()) else {
                continue;
            };
            let (Ok(pid), Ok(u), Ok(v)) = (
                pid_s.trim().parse::<u32>(),
                u_s.trim().parse::<f64>(),
                v_s.trim().parse::<f64>(),
            ) else {
                continue;
            };
            per_pid.insert(pid, (u.min(100.0) as f32, v.max(0.0) as u64));
        }

        Some(GpuInfo {
            usage_pct: usage,
            vram_used_bytes: used,
            vram_total_bytes: limit,
            per_pid,
        })
    }

    fn battery_info(w: &WMIConnection) -> Option<BatteryStat> {
        let rows = w
            .raw_query::<BatteryRow>(
                "SELECT EstimatedChargeRemaining, BatteryStatus FROM Win32_Battery",
            )
            .ok()?;
        let row = rows.into_iter().next()?;
        let percent = row.EstimatedChargeRemaining?.min(100) as u8;
        // BatteryStatus: 1 = discharging, 2 = on AC (charging or full)
        let charging = row.BatteryStatus.map_or(true, |s| s == 2);
        Some(BatteryStat { percent, charging })
    }
}

impl Default for SystemStatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: run a single collection with a short wait (first CPU sample
/// needs a previous one to compute a delta).
pub fn collect_with_delay() -> SystemStats {
    let mut c = SystemStatsCollector::new();
    std::thread::sleep(Duration::from_millis(300));
    c.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_runs() {
        let mut c = SystemStatsCollector::new();
        let s = c.collect();
        assert!(s.ram_total_bytes > 0);
        assert!(s.uptime_secs > 0);
        println!("cpu={:.1}% ram={}/{} swap={}/{} disks={:?} gpu={:?} battery={:?} procs={} per_pid={}",
            s.cpu_usage, s.ram_used_bytes, s.ram_total_bytes, s.swap_used_bytes,
            s.swap_total_bytes, s.disks, s.gpu, s.battery, s.process_count, s.processes.len());
    }

    #[test]
    fn test_disk_pct() {
        let d = DiskStat { label: "C:".into(), used_bytes: 50, total_bytes: 200 };
        assert_eq!(d.usage_pct(), 25.0);
        let e = DiskStat { label: "D:".into(), used_bytes: 0, total_bytes: 0 };
        assert_eq!(e.usage_pct(), 0.0);
    }
}
