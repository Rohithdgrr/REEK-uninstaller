import { useEffect, useState } from "react";
import {
  Cpu,
  MemoryStick,
  HardDrive,
  Battery,
  Zap,
  Activity,
  Clock3,
  Layers,
  Gauge,
  BatteryCharging,
  Flame,
} from "lucide-react";
import { getSystemStats, type SystemStatsDto } from "../lib/tauri";

function fmtBytes(b: number) {
  if (b === 0) return "—";
  const u = ["B", "KiB", "MiB", "GiB", "TiB"];
  let i = 0;
  let v = b;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(i === 0 ? 0 : 1)} ${u[i]}`;
}

function fmtUptime(secs: number) {
  const h = Math.floor(secs / 3600);
  if (h < 24) return `${h}h`;
  const d = Math.floor(h / 24);
  const rh = h % 24;
  return `${d}d ${rh}h`;
}

function pctBarColor(p: number) {
  if (p >= 90) return "from-[#FF2047] to-[#FF3B6A] shadow-[0_0_12px_rgba(255,32,71,0.5)]";
  if (p >= 75) return "from-[#FF801F] to-[#FF9A3D] shadow-[0_0_10px_rgba(255,128,31,0.4)]";
  if (p >= 50) return "from-[#E11D48] to-[#FF3B6A] shadow-[0_0_10px_rgba(225,29,72,0.35)]";
  return "from-[#E11D48] to-[#C9A84C] shadow-[0_0_8px_rgba(225,29,72,0.2)]";
}

function pctText(p: number) {
  if (p >= 90) return "text-[#FF2047]";
  if (p >= 75) return "text-[#FF801F]";
  return "text-[#F5F0EB]";
}

export function SystemStatsBar() {
  const [s, setS] = useState<SystemStatsDto | null>(null);
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const d = await getSystemStats();
        if (alive) setS(d);
      } catch {}
    };
    poll();
    const id = setInterval(poll, 2500);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  if (!s) {
    return (
      <div className="p-5 animate-pulse">
        <div className="h-4 w-32 bg-[rgba(225,29,72,0.08)] rounded-full" />
        <div className="mt-4 grid grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="h-[118px] bg-[#1A1A1A] rounded-[12px] border border-[rgba(225,29,72,0.04)]" />
          ))}
        </div>
      </div>
    );
  }

  const cpuStatus = s.cpu >= 85 ? "Under Stress" : s.cpu >= 50 ? "Active" : s.cpu >= 15 ? "Balanced" : "Idle";
  const ramStatus = s.ram_pct >= 85 ? "High Pressure" : s.ram_pct >= 60 ? "Elevated" : "Healthy";

  return (
    <div className="bg-[#141414] p-3 md:p-4 flex gap-3 md:gap-4 items-stretch">
      {/* Mahakali logo — BIG, equal height to the entire Vitality box, NOT inside */}
      <div className="hidden lg:flex w-[300px] xl:w-[340px] shrink-0 rounded-[16px] overflow-hidden bg-black border border-[rgba(225,29,72,0.16)] shadow-[0_0_30px_rgba(225,29,72,0.14),0_0_60px_rgba(225,29,72,0.06)] flex-col">
        <img src="/mahakali.png" alt="Mahakali" className="w-full h-full object-cover object-center scale-[1.04]" />
      </div>

      {/* Vitality box — beside logo, same height via flex stretch */}
      <div className="flex-1 min-w-0 rounded-[16px] border border-[rgba(225,29,72,0.08)] bg-[#141414] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="px-5 pt-4 pb-3 flex items-center justify-between border-b border-[rgba(225,29,72,0.06)] bg-[radial-gradient(600px_120px_at_20%_0%,rgba(225,29,72,0.06),transparent_70%)]">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-[8px] bg-[rgba(225,29,72,0.12)] border border-[rgba(225,29,72,0.18)] flex items-center justify-center text-[#E11D48]">
              <Gauge size={16} />
            </div>
            <div>
              <h3 className="text-[12px] font-semibold tracking-[0.18em] uppercase text-[#F5F0EB] leading-none">System Vitality</h3>
              <p className="text-[11px] text-[#6B6661] tracking-[0.08em] uppercase mt-0.5">Live telemetry • 2.5s refresh</p>
            </div>
          </div>
          <div className="hidden sm:flex items-center gap-3 text-[11px]">
            <span className="inline-flex items-center gap-1.5 text-[#A8A39E] bg-[#1A1A1A] border border-[rgba(225,29,72,0.06)] rounded-full px-3 py-1">
              <Activity size={12} className="text-[#E11D48]" /> {s.process_count} procs
            </span>
            <span className="inline-flex items-center gap-1.5 text-[#A8A39E] bg-[#1A1A1A] border border-[rgba(225,29,72,0.06)] rounded-full px-3 py-1">
              <Clock3 size={12} className="text-[#C9A84C]" /> up {fmtUptime(s.uptime_secs)}
            </span>
          </div>
        </div>

      {/* Main 4-card grid */}
      <div className="p-4 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* CPU */}
        <div className="relative rounded-[12px] bg-[#1A1A1A] border border-[rgba(225,29,72,0.06)] p-4 overflow-hidden group hover:border-[rgba(225,29,72,0.14)] hover:shadow-[0_0_24px_rgba(225,29,72,0.08)] transition-all">
          <div className="absolute -top-6 -right-6 w-24 h-24 bg-[radial-gradient(circle_at_center,rgba(225,29,72,0.14),transparent_70%)] pointer-events-none" />
          <div className="flex items-start justify-between relative">
            <div className="w-10 h-10 rounded-[10px] bg-[rgba(225,29,72,0.12)] border border-[rgba(225,29,72,0.18)] flex items-center justify-center text-[#E11D48]">
              <Cpu size={18} />
            </div>
            <span className="text-[10px] font-medium tracking-[0.14em] uppercase text-[#6B6661] bg-black border border-[rgba(225,29,72,0.06)] rounded-full px-2.5 py-1">
              Processor
            </span>
          </div>
          <div className="mt-4 relative">
            <div className={`text-[30px] font-bold leading-none tracking-[-0.02em] ${pctText(s.cpu)}`}>
              {s.cpu.toFixed(1)}<span className="text-[16px] font-medium text-[#6B6661] ml-0.5">%</span>
            </div>
            <div className="mt-1 flex items-center gap-1.5 text-[11px]">
              <Flame size={12} className={s.cpu > 70 ? "text-[#FF2047]" : "text-[#6B6661]"} />
              <span className={s.cpu > 70 ? "text-[#FF8DA0]" : "text-[#A8A39E]"}>{cpuStatus}</span>
            </div>
          </div>
          <div className="mt-3 h-1.5 rounded-full bg-[rgba(255,255,255,0.06)] overflow-hidden p-[1px]">
            <div
              className={`h-full rounded-full bg-gradient-to-r transition-all duration-700 ease-out ${pctBarColor(s.cpu)}`}
              style={{ width: `${Math.min(s.cpu, 100)}%` }}
            />
          </div>
        </div>

        {/* RAM */}
        <div className="relative rounded-[12px] bg-[#1A1A1A] border border-[rgba(225,29,72,0.06)] p-4 overflow-hidden group hover:border-[rgba(225,29,72,0.14)] hover:shadow-[0_0_24px_rgba(225,29,72,0.08)] transition-all">
          <div className="absolute -top-6 -right-6 w-24 h-24 bg-[radial-gradient(circle_at_center,rgba(201,168,76,0.12),transparent_70%)] pointer-events-none" />
          <div className="flex items-start justify-between relative">
            <div className="w-10 h-10 rounded-[10px] bg-[rgba(201,168,76,0.12)] border border-[rgba(201,168,76,0.18)] flex items-center justify-center text-[#C9A84C]">
              <MemoryStick size={18} />
            </div>
            <span className="text-[10px] font-medium tracking-[0.14em] uppercase text-[#6B6661] bg-black border border-[rgba(225,29,72,0.06)] rounded-full px-2.5 py-1">
              Memory
            </span>
          </div>
          <div className="mt-4 relative">
            <div className={`text-[30px] font-bold leading-none tracking-[-0.02em] ${pctText(s.ram_pct)}`}>
              {s.ram_pct.toFixed(0)}<span className="text-[16px] font-medium text-[#6B6661] ml-0.5">%</span>
            </div>
            <div className="mt-1 text-[11px] text-[#A8A39E]">
              {fmtBytes(s.ram_used)} <span className="text-[#6B6661]">/</span> {fmtBytes(s.ram_total)} • {ramStatus}
            </div>
          </div>
          <div className="mt-3 h-1.5 rounded-full bg-[rgba(255,255,255,0.06)] overflow-hidden p-[1px]">
            <div
              className={`h-full rounded-full bg-gradient-to-r transition-all duration-700 ease-out ${pctBarColor(s.ram_pct)}`}
              style={{ width: `${Math.min(s.ram_pct, 100)}%` }}
            />
          </div>
          {s.swap_total > 0 && (
            <div className="mt-2 flex items-center justify-between text-[10px] text-[#6B6661]">
              <span className="inline-flex items-center gap-1">
                <Layers size={10} /> Swap
              </span>
              <span>
                {fmtBytes(s.swap_used)} / {fmtBytes(s.swap_total)}
              </span>
            </div>
          )}
        </div>

        {/* GPU */}
        <div className="relative rounded-[12px] bg-[#1A1A1A] border border-[rgba(225,29,72,0.06)] p-4 overflow-hidden group hover:border-[rgba(225,29,72,0.14)] hover:shadow-[0_0_24px_rgba(225,29,72,0.08)] transition-all">
          <div className="absolute -top-6 -right-6 w-24 h-24 bg-[radial-gradient(circle_at_center,rgba(255,128,31,0.12),transparent_70%)] pointer-events-none" />
          <div className="flex items-start justify-between relative">
            <div className="w-10 h-10 rounded-[10px] bg-[rgba(255,128,31,0.12)] border border-[rgba(255,128,31,0.18)] flex items-center justify-center text-[#FF801F]">
              <Zap size={18} />
            </div>
            <span className="text-[10px] font-medium tracking-[0.14em] uppercase text-[#6B6661] bg-black border border-[rgba(225,29,72,0.06)] rounded-full px-2.5 py-1">
              Graphics
            </span>
          </div>
          {s.gpu ? (
            <>
              <div className="mt-4 relative">
                <div className={`text-[30px] font-bold leading-none tracking-[-0.02em] ${pctText(s.gpu.usage)}`}>
                  {s.gpu.usage.toFixed(0)}<span className="text-[16px] font-medium text-[#6B6661] ml-0.5">%</span>
                </div>
                <div className="mt-1 text-[11px] text-[#A8A39E] truncate pr-2" title={s.gpu.name}>
                  {s.gpu.name.split(" ").slice(0, 3).join(" ")}
                </div>
              </div>
              <div className="mt-3 h-1.5 rounded-full bg-[rgba(255,255,255,0.06)] overflow-hidden p-[1px]">
                <div
                  className={`h-full rounded-full bg-gradient-to-r transition-all duration-700 ease-out ${pctBarColor(s.gpu.usage)}`}
                  style={{ width: `${Math.min(s.gpu.usage, 100)}%` }}
                />
              </div>
              <div className="mt-2 flex items-center justify-between text-[10px] text-[#6B6661]">
                <span>VRAM</span>
                <span>
                  {fmtBytes(s.gpu.vram_used)} / {fmtBytes(s.gpu.vram_total)}
                </span>
              </div>
            </>
          ) : (
            <>
              <div className="mt-4 relative">
                <div className="text-[22px] font-semibold leading-none text-[#6B6661]">—</div>
                <div className="mt-1 text-[11px] text-[#6B6661]">No discrete GPU detected</div>
              </div>
              <div className="mt-3 h-1.5 rounded-full bg-[rgba(255,255,255,0.04)]" />
              <div className="mt-2 text-[10px] text-[#6B6661]">Integrated graphics</div>
            </>
          )}
        </div>

        {/* Battery + Uptime */}
        <div className="relative rounded-[12px] bg-[#080808] border border-[rgba(225,29,72,0.06)] p-4 overflow-hidden group hover:border-[rgba(225,29,72,0.14)] transition-all">
          <div className="absolute inset-0 bg-[radial-gradient(300px_100px_at_80%_0%,rgba(225,29,72,0.06),transparent_70%)] pointer-events-none" />
          <div className="relative">
            {s.battery ? (
              <>
                <div className="flex items-start justify-between">
                  <div
                    className={`w-10 h-10 rounded-[10px] border flex items-center justify-center ${
                      s.battery.charging
                        ? "bg-[rgba(17,255,153,0.12)] border-[rgba(17,255,153,0.2)] text-[#11FF99]"
                        : s.battery.percent < 20
                          ? "bg-[rgba(255,32,71,0.12)] border-[rgba(255,32,71,0.18)] text-[#FF2047]"
                          : "bg-[rgba(225,29,72,0.08)] border-[rgba(225,29,72,0.12)] text-[#E11D48]"
                    }`}
                  >
                    {s.battery.charging ? <BatteryCharging size={18} /> : <Battery size={18} />}
                  </div>
                  <span
                    className={`text-[10px] font-medium tracking-[0.14em] uppercase rounded-full px-2.5 py-1 border ${
                      s.battery.charging
                        ? "bg-[rgba(17,255,153,0.12)] border-[rgba(17,255,153,0.2)] text-[#11FF99]"
                        : "bg-black border-[rgba(225,29,72,0.06)] text-[#6B6661]"
                    }`}
                  >
                    {s.battery.charging ? "Charging" : "Battery"}
                  </span>
                </div>
                <div className="mt-4">
                  <div className="flex items-baseline gap-1">
                    <span className={`text-[30px] font-bold leading-none tracking-[-0.02em] ${s.battery.percent < 20 ? "text-[#FF2047]" : "text-[#F5F0EB]"}`}>
                      {s.battery.percent}
                      <span className="text-[16px] font-medium text-[#6B6661]">%</span>
                    </span>
                    {s.battery.charging && <span className="text-[#11FF99] text-xs">⚡</span>}
                  </div>
                  <div className="mt-1 text-[11px] text-[#A8A39E]">{s.battery.charging ? "Plugged in • Optimizing" : s.battery.percent < 20 ? "Low • Plug in soon" : "Discharging"}</div>
                </div>
                <div className="mt-3 relative h-2.5 rounded-full bg-[rgba(255,255,255,0.06)] overflow-hidden p-[2px] flex items-center">
                  <div
                    className={`h-full rounded-full transition-all duration-700 ${s.battery.charging ? "bg-gradient-to-r from-[#11FF99] to-[#00D68B] shadow-[0_0_10px_rgba(17,255,153,0.4)]" : s.battery.percent < 20 ? "bg-gradient-to-r from-[#FF2047] to-[#FF3B6A]" : "bg-gradient-to-r from-[#E11D48] to-[#C9A84C]"}`}
                    style={{ width: `${Math.min(s.battery.percent, 100)}%` }}
                  />
                </div>
              </>
            ) : (
              <>
                <div className="flex items-start justify-between">
                  <div className="w-10 h-10 rounded-[10px] bg-black border border-[rgba(225,29,72,0.06)] flex items-center justify-center text-[#6B6661]">
                    <Battery size={18} />
                  </div>
                  <span className="text-[10px] font-medium tracking-[0.14em] uppercase text-[#6B6661] bg-black border border-[rgba(225,29,72,0.06)] rounded-full px-2.5 py-1">Power</span>
                </div>
                <div className="mt-4">
                  <div className="text-[16px] font-semibold text-[#A8A39E]">AC Power</div>
                  <div className="mt-1 text-[11px] text-[#6B6661]">No battery • Desktop</div>
                </div>
                <div className="mt-3 h-2.5 rounded-full bg-[rgba(255,255,255,0.04)]" />
              </>
            )}
            <div className="mt-3 grid grid-cols-2 gap-2 pt-3 border-t border-[rgba(225,29,72,0.06)]">
              <div className="rounded-[8px] bg-[#141414] border border-[rgba(225,29,72,0.04)] px-2.5 py-2">
                <div className="text-[10px] tracking-[0.12em] uppercase text-[#6B6661]">Uptime</div>
                <div className="text-[13px] font-semibold text-[#F5F0EB] mt-0.5">{fmtUptime(s.uptime_secs)}</div>
              </div>
              <div className="rounded-[8px] bg-[#141414] border border-[rgba(225,29,72,0.04)] px-2.5 py-2">
                <div className="text-[10px] tracking-[0.12em] uppercase text-[#6B6661]">Processes</div>
                <div className="text-[13px] font-semibold text-[#F5F0EB] mt-0.5">{s.process_count}</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Disks — detailed */}
      <div className="px-4 pb-4">
        <div className="rounded-[12px] bg-[#1A1A1A] border border-[rgba(225,29,72,0.06)] p-4">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <div className="w-7 h-7 rounded-[8px] bg-[rgba(225,29,72,0.08)] border border-[rgba(225,29,72,0.12)] flex items-center justify-center text-[#E11D48]">
                <HardDrive size={14} />
              </div>
              <h4 className="text-[11px] font-semibold tracking-[0.14em] uppercase text-[#F5F0EB]">Storage</h4>
              <span className="text-[11px] text-[#6B6661]">• {s.disks.length} volumes</span>
            </div>
            <span className="text-[11px] text-[#6B6661] hidden sm:inline">{s.disks.reduce((a, d) => a + d.used, 0) > 0 ? `${fmtBytes(s.disks.reduce((a, d) => a + d.used, 0))} used` : ""}</span>
          </div>

          {s.disks.length === 0 ? (
            <div className="text-[12px] text-[#6B6661] py-2">No disk data available</div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {s.disks.map((d) => (
                <div
                  key={d.label}
                  className="rounded-[10px] bg-[#0A0A0A] border border-[rgba(225,29,72,0.06)] p-3 hover:border-[rgba(225,29,72,0.12)] transition-colors"
                >
                  <div className="flex items-center justify-between">
                    <span className="inline-flex items-center gap-1.5 text-[12px] font-semibold text-[#F5F0EB]">
                      <span className="w-6 h-6 rounded-[6px] bg-[#141414] border border-[rgba(225,29,72,0.08)] flex items-center justify-center text-[10px] font-bold text-[#C9A84C]">
                        {d.label.replace(":", "").slice(0, 2)}
                      </span>
                      {d.label}
                    </span>
                    <span className={`text-[12px] font-bold ${pctText(d.pct)}`}>{d.pct.toFixed(0)}%</span>
                  </div>
                  <div className="mt-2.5 h-1.5 rounded-full bg-[rgba(255,255,255,0.06)] overflow-hidden p-[1px]">
                    <div
                      className={`h-full rounded-full bg-gradient-to-r transition-all duration-700 ${pctBarColor(d.pct)}`}
                      style={{ width: `${Math.min(d.pct, 100)}%` }}
                    />
                  </div>
                  <div className="mt-2 flex items-center justify-between text-[11px]">
                    <span className="text-[#A8A39E]">{fmtBytes(d.used)} used</span>
                    <span className="text-[#6B6661]">{fmtBytes(d.total)} total</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
      </div>
    </div>
  );
}
