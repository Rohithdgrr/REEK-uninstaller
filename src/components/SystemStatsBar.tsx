import { useEffect, useState } from "react";
import { Cpu, MemoryStick, HardDrive, Battery, Zap, Activity } from "lucide-react";
import { getSystemStats, type SystemStatsDto } from "../lib/tauri";

function fmtBytes(b: number) {
  if (b === 0) return "—";
  const u = ["B","KiB","MiB","GiB","TiB"];
  let i = 0; let v = b;
  while (v >= 1024 && i < u.length-1) { v/=1024; i++; }
  return `${v.toFixed(i===0?0:1)} ${u[i]}`;
}
function pctColor(p: number) {
  if (p > 90) return "text-red-600";
  if (p > 75) return "text-amber-600";
  return "text-slate-700";
}

export function SystemStatsBar() {
  const [s, setS] = useState<SystemStatsDto | null>(null);
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try { const d = await getSystemStats(); if (alive) setS(d); } catch {}
    };
    poll();
    const id = setInterval(poll, 2500);
    return () => { alive = false; clearInterval(id); };
  }, []);

  if (!s) return <div className="h-10 bg-white border border-slate-200 rounded-xl animate-pulse" />;
  return (
    <div className="bg-white border border-slate-200 rounded-xl shadow-sm px-4 py-2.5 flex flex-wrap items-center gap-4 text-xs">
      <span className="inline-flex items-center gap-1.5 font-medium text-slate-700">
        <Cpu size={14} className="text-blue-600" /> CPU <span className={pctColor(s.cpu)}>{s.cpu.toFixed(1)}%</span>
      </span>
      <span className="h-4 w-px bg-slate-200 hidden sm:block" />
      <span className="inline-flex items-center gap-1.5 font-medium text-slate-700">
        <MemoryStick size={14} className="text-blue-600" /> RAM <span className={pctColor(s.ram_pct)}>{s.ram_pct.toFixed(0)}%</span>
        <span className="text-slate-500">{fmtBytes(s.ram_used)} / {fmtBytes(s.ram_total)}</span>
      </span>
      <span className="h-4 w-px bg-slate-200 hidden sm:block" />
      <span className="inline-flex items-center gap-1.5 font-medium text-slate-700">
        <HardDrive size={14} className="text-blue-600" /> Disks
        <span className="text-slate-500 hidden md:inline">
          {s.disks.map(d => `${d.label} ${d.pct.toFixed(0)}%`).join(" · ") || "—"}
        </span>
      </span>
      {s.gpu && (
        <>
          <span className="h-4 w-px bg-slate-200 hidden sm:block" />
          <span className="inline-flex items-center gap-1.5 font-medium text-slate-700">
            <Zap size={14} className="text-blue-600" /> GPU {s.gpu.name.split(" ").slice(0,2).join(" ")} <span className={pctColor(s.gpu.usage)}>{s.gpu.usage.toFixed(0)}%</span>
          </span>
        </>
      )}
      {s.battery && (
        <>
          <span className="h-4 w-px bg-slate-200 hidden sm:block" />
          <span className="inline-flex items-center gap-1.5 font-medium text-slate-700">
            <Battery size={14} className={s.battery.charging ? "text-green-600" : "text-slate-500"} /> {s.battery.percent}%{s.battery.charging ? " ⚡" : ""}
          </span>
        </>
      )}
      <span className="ml-auto inline-flex items-center gap-1.5 text-slate-500">
        <Activity size={12} /> {s.process_count} procs · up {Math.floor(s.uptime_secs/3600)}h
      </span>
    </div>
  );
}
