import { useEffect, useState } from "react";
import { ArrowUpDown, Package, Check, Minus, ChevronRight, Sparkles } from "lucide-react";
import type { AppEntry, AppResourceDto } from "../lib/tauri";
import { getAppIcon, getAppResources } from "../lib/tauri";
import { useAppStore } from "../store/useAppStore";

export function AppTable({ apps, onDetails }: { apps: AppEntry[]; onDetails?: (id: string) => void }) {
  const { selected, toggleSelect, toggleSelectAll, sortKey, sortDir, setSort } = useAppStore();
  const visibleIds = apps.map((a) => a.id);
  const allChecked = visibleIds.length > 0 && visibleIds.every((id) => selected.has(id));
  const indeterminate = !allChecked && visibleIds.some((id) => selected.has(id));
  const [resources, setResources] = useState<Record<string, AppResourceDto>>({});

  useEffect(() => {
    let alive = true;
    const fetchRes = async () => {
      try {
        const m = await getAppResources();
        if (alive) setResources(m);
      } catch {}
    };
    fetchRes();
    const id = setInterval(fetchRes, 3000);
    return () => { alive = false; clearInterval(id); };
  }, [apps.length]);

  return (
    <div className="overflow-hidden rounded-[16px] border border-[rgba(225,29,72,0.08)] bg-[#0A0A0A]">
      {/* Simple, tactile header */}
      <div className="sticky top-0 z-10 bg-[#141414]/90 backdrop-blur-[8px] border-b border-[rgba(225,29,72,0.06)] px-3 md:px-4 py-3 flex items-center gap-3">
        <button
          aria-label="Select all"
          onClick={() => toggleSelectAll(visibleIds)}
          className={`custom-checkbox ${allChecked ? "checked" : indeterminate ? "indeterminate" : ""} shrink-0`}
        >
          {allChecked ? <Check size={10} strokeWidth={3} /> : indeterminate ? <Minus size={10} strokeWidth={3} /> : null}
        </button>

        <div className="flex-1 min-w-0 flex items-center gap-2">
          <span className="hidden sm:inline text-[11px] font-medium tracking-[0.12em] uppercase text-[#6B6661]">Select</span>
          <span className="hidden sm:inline text-[11px] text-[#4A4540]">•</span>
          <span className="text-[11px] text-[#A8A39E] truncate">{visibleIds.length} apps • click card for detail • check to select</span>
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
          <SortPill label="Heavy" active={sortKey === "size"} dir={sortDir} onClick={() => setSort("size")} hint="Storage size + live CPU/GPU/RAM tie-break — default heaviest first" />
          <SortPill label="Usage" active={sortKey === "resources"} dir={sortDir} onClick={() => setSort("resources")} className="hidden sm:inline-flex" hint="Live CPU/GPU/RAM/VRAM + running" />
          <SortPill label="Name" active={sortKey === "name"} dir={sortDir} onClick={() => setSort("name")} className="hidden lg:inline-flex" />
          <SortPill label="Date" active={sortKey === "date"} dir={sortDir} onClick={() => setSort("date")} className="hidden lg:inline-flex" />
        </div>
      </div>

      {/* List */}
      <div className="max-h-[54vh] overflow-auto divide-y divide-[rgba(225,29,72,0.04)]">
        {apps.length === 0 ? (
          <div className="px-6 py-16 text-center">
            <div className="mx-auto w-14 h-14 rounded-[16px] bg-[#141414] border border-[rgba(225,29,72,0.08)] flex items-center justify-center text-[#6B6661] shadow-[0_0_24px_rgba(225,29,72,0.06)]">
              <Package size={22} />
            </div>
            <p className="mt-4 text-[14px] font-medium text-[#F5F0EB]">No applications found</p>
            <p className="text-[12px] text-[#6B6661] mt-1">Try another search or hit Scan to refresh.</p>
          </div>
        ) : (
          apps.map((app, idx) => {
            const isSelected = selected.has(app.id);
            const res = resources[app.id];
            const running = res?.is_running;
            return (
              <div
                key={app.id}
                onClick={() => onDetails?.(app.id)}
                className={`group relative flex items-center gap-3 md:gap-4 px-3 md:px-4 py-3.5 cursor-pointer transition-all duration-150
                  ${isSelected ? "bg-[rgba(225,29,72,0.07)]" : "bg-transparent hover:bg-[#141414]"}
                `}
                style={{ animation: `slideUpFade 180ms ease ${Math.min(idx, 8) * 22}ms both` }}
                title="Click for details"
              >
                {/* left accent */}
                <span className={`absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-7 rounded-full transition-all ${isSelected ? "bg-[#E11D48] opacity-100" : "bg-transparent group-hover:bg-[rgba(225,29,72,0.25)] opacity-0 group-hover:opacity-100"}`} />

                <button
                  aria-label={`Select ${app.name}`}
                  onClick={(e) => { e.stopPropagation(); toggleSelect(app.id); }}
                  className={`custom-checkbox shrink-0 ${isSelected ? "checked" : ""}`}
                >
                  {isSelected && <Check size={10} strokeWidth={3} />}
                </button>

                <AppIcon app={app} running={running} selected={isSelected} />

                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 min-w-0">
                    <p className="text-[14px] font-medium leading-none text-[#F5F0EB] truncate">{app.name}</p>
                    {running && <span className="inline-flex items-center gap-1 text-[10px] font-medium tracking-[0.08em] uppercase bg-[rgba(17,255,153,0.12)] border border-[rgba(17,255,153,0.22)] text-[#11FF99] rounded-full px-1.5 py-0.5 shrink-0"><span className="w-1 h-1 rounded-full bg-[#11FF99] animate-pulse" /> Live</span>}
                    {isSelected && <Sparkles size={12} className="text-[#E11D48] opacity-60 shrink-0 hidden sm:inline" />}
                  </div>
                  <p className="text-[12px] text-[#6B6661] truncate mt-1">{app.publisher ?? app.source_label}</p>
                </div>

                {/* meta pills — simple, not crowded */}
                <div className="hidden md:flex items-center gap-2 shrink-0">
                  <span className="hidden lg:inline-flex max-w-[140px] truncate rounded-full bg-[#141414] border border-[rgba(225,29,72,0.08)] px-2.5 py-1 text-[11px] font-medium text-[#A8A39E]">{app.version ?? "—"}</span>
                  <span className={`inline-flex rounded-full border px-2.5 py-1 text-[11px] font-medium ${pickSizeStyle(app.size_display)}`}>{app.size_display ?? "—"}</span>
                  <span className="hidden xl:inline-flex text-[11px] text-[#6B6661] w-[88px] justify-end">{formatDate(app.install_date)}</span>
                </div>

                {/* mobile size badge */}
                <span className={`md:hidden shrink-0 rounded-full border px-2 py-1 text-[11px] font-medium ${pickSizeStyle(app.size_display)}`}>{app.size_display ? app.size_display.split(" ")[0] : "—"}</span>

                <ChevronRight size={14} className="text-[#4A4540] group-hover:text-[#A8A39E] group-hover:translate-x-0.5 transition-all shrink-0" />
              </div>
            );
          })
        )}
      </div>

      {/* bottom fade */}
      <div className="h-px bg-gradient-to-r from-transparent via-[rgba(225,29,72,0.12)] to-transparent" />
    </div>
  );
}

function SortPill({ label, active, dir, onClick, className = "", hint }: { label: string; active: boolean; dir: string; onClick: () => void; className?: string; hint?: string }) {
  return (
    <button
      onClick={onClick}
      title={hint}
      className={`inline-flex items-center gap-1 rounded-full border px-2.5 py-1 text-[11px] font-medium transition-colors ${className} ${
        active ? "bg-[rgba(225,29,72,0.14)] border-[rgba(225,29,72,0.22)] text-[#F5F0EB] shadow-[0_0_12px_rgba(225,29,72,0.12)]" : "bg-[#0A0A0A] border-[rgba(225,29,72,0.06)] text-[#6B6661] hover:text-[#A8A39E] hover:border-[rgba(225,29,72,0.12)]"
      }`}
    >
      {label} <ArrowUpDown size={11} className={active ? (dir === "asc" ? "text-[#E11D48]" : "text-[#E11D48] rotate-180") : "text-[#4A4540]"} />
    </button>
  );
}

function pickSizeStyle(size?: string | null) {
  if (!size || size === "—") return "bg-[#0A0A0A] border-[rgba(225,29,72,0.06)] text-[#6B6661]";
  // large apps -> warmer
  const isLarge = size.includes("GB");
  return isLarge
    ? "bg-[rgba(255,128,31,0.08)] border-[rgba(255,128,31,0.14)] text-[#FF9A3D]"
    : "bg-[#141414] border-[rgba(225,29,72,0.08)] text-[#A8A39E]";
}

function formatDate(d?: string | null) {
  if (!d || d === "—") return "—";
  // already YYYY-MM-DD, keep as is but subtle
  return d;
}

function AppIcon({ app, running, selected }: { app: AppEntry; running?: boolean; selected: boolean }) {
  const [b64, setB64] = useState<string | null>(null);
  const hasIcon = !!app.icon_path;

  useEffect(() => {
    if (!hasIcon) return;
    let cancelled = false;
    getAppIcon(app.id)
      .then((v) => { if (!cancelled && v) setB64(v); })
      .catch(() => {});
    return () => { cancelled = true; };
  }, [app.id, hasIcon]);

  const base = "w-10 h-10 rounded-[12px] shrink-0 flex items-center justify-center overflow-hidden relative";

  if (b64) {
    return (
      <div className={`${base} border ${selected ? "border-[rgba(225,29,72,0.22)] shadow-[0_0_16px_rgba(225,29,72,0.12)]" : "border-[rgba(225,29,72,0.08)]"} bg-[#141414]`}>
        <img src={`data:image/png;base64,${b64}`} alt="" className="w-full h-full object-contain p-1" loading="lazy" onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }} />
        {running && <span className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-[#11FF99] border-2 border-[#0A0A0A] shadow-[0_0_6px_rgba(17,255,153,0.7)]" />}
      </div>
    );
  }

  if (app.icon_color) {
    const bg = `rgb(${app.icon_color})`;
    return (
      <div className={`${base} border ${selected ? "border-[rgba(225,29,72,0.22)]" : "border-[rgba(225,29,72,0.08)]"} text-white text-[11px] font-bold`} style={{ backgroundColor: bg }}>
        {app.name.slice(0, 2).toUpperCase()}
        {running && <span className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-[#11FF99] border-2 border-[#0A0A0A]" />}
      </div>
    );
  }

  return (
    <div className={`${base} bg-[#1A1A1A] border ${selected ? "border-[rgba(225,29,72,0.18)]" : "border-[rgba(225,29,72,0.06)]"} text-[#A8A39E] text-[11px] font-bold`}>
      {app.name.slice(0, 2).toUpperCase()}
      {running && <span className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-[#11FF99] border-2 border-[#0A0A0A]" />}
    </div>
  );
}
