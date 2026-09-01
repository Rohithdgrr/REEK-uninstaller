import { useEffect, useState, useMemo } from "react";
import { X, FolderOpen, Trash2, Flame, Search, ShieldAlert, Cpu, MemoryStick, Activity, Zap, HardDrive, Folder, File, AlertTriangle, CheckCircle2, Loader2, Database, Layers, Clock3, Link2, Trash, Package, FileWarning, Copy } from "lucide-react";
import { getAppDetails, analyzeLeftovers, type AppDetails, type LeftoverDto, uninstallApplications, getAppIcon, getAppResource, type AppResourceDto } from "../lib/tauri";

function categorizePath(p: string): { label: string; drive: string } {
  const lower = p.toLowerCase();
  let label = "Other";
  if (lower.includes("program files")) label = "Program Files";
  else if (lower.includes("appdata")) {
    if (lower.includes("appdata\\local") || lower.includes("appdata/local")) label = "AppData \\ Local";
    else if (lower.includes("appdata\\roaming") || lower.includes("appdata/roaming")) label = "AppData \\ Roaming";
    else label = "AppData";
  } else if (lower.includes("\\users\\") || lower.includes("/users/")) label = "Users";
  else if (lower.includes("programdata")) label = "ProgramData";
  else if (lower.includes("\\windows") || lower.includes("/windows")) label = "Windows";
  const drive = p.match(/^([A-Za-z]:)/)?.[1]?.toUpperCase() ?? (p.startsWith("HK") ? "Registry" : "—");
  return { label, drive };
}

function safetyBadge(s: string) {
  const v = s.toLowerCase();
  if (v === "safe") return "bg-[rgba(17,255,153,0.12)] text-[#11FF99] border-[rgba(17,255,153,0.25)]";
  if (v === "critical") return "bg-[rgba(255,32,71,0.12)] text-[#FF3B6A] border-[rgba(255,32,71,0.25)]";
  if (v === "dangerous") return "bg-[rgba(255,128,31,0.12)] text-[#FF801F] border-[rgba(255,128,31,0.25)]";
  return "bg-[rgba(255,193,7,0.12)] text-[#FFC107] border-[rgba(255,193,7,0.25)]";
}

function artifactCategory(t: string, description?: string): string {
  // Duplicate installers have File type but description contains "Duplicate"
  if (description && description.toLowerCase().includes("duplicate")) return "Duplicates";
  const v = t.toLowerCase();
  if (v === "directory") return "Folders";
  if (v === "file") return "Files";
  if (v === "tempfile") return "Junk";
  if (v === "registrykey" || v === "registryvalue") return "Registry";
  if (v === "service") return "Services";
  if (v === "scheduledtask") return "Tasks";
  if (v === "shortcut") return "Shortcuts";
  if (v === "shellextension" || v === "driver" || v === "font") return "Modules";
  return "Other";
}

function categoryIcon(cat: string) {
  switch(cat) {
    case "Folders": return <Folder size={12}/>;
    case "Files": return <File size={12}/>;
    case "Duplicates": return <Copy size={12}/>;
    case "Junk": return <Trash size={12}/>;
    case "Registry": return <Database size={12}/>;
    case "Services": return <Layers size={12}/>;
    case "Tasks": return <Clock3 size={12}/>;
    case "Shortcuts": return <Link2 size={12}/>;
    case "Modules": return <Package size={12}/>;
    default: return <FileWarning size={12}/>;
  }
}

export function AppDetailsDrawer({
  id, onClose, onUninstalled,
}: { id: string | null; onClose: () => void; onUninstalled: () => void; }) {
  const [d, setD] = useState<AppDetails | null>(null);
  const [leftovers, setLeftovers] = useState<LeftoverDto[] | null>(null);
  const [leftLoading, setLeftLoading] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [iconB64, setIconB64] = useState<string | null>(null);
  const [res, setRes] = useState<AppResourceDto | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [activeCat, setActiveCat] = useState<string>("All");

  useEffect(() => {
    if (!id) return;
    setD(null); setLeftovers(null); setErr(null); setIconB64(null); setRes(null); setShowDeleteConfirm(false); setLeftLoading(false); setActiveCat("All");
    getAppDetails(id).then(setD).catch(e => setErr(String(e)));
  }, [id]);

  useEffect(() => {
    if (!d?.icon_path || !id) return;
    let cancelled = false;
    getAppIcon(id).then(v => { if (!cancelled && v) setIconB64(v); }).catch(()=>{});
    return () => { cancelled = true; };
  }, [d?.icon_path, id]);

  useEffect(() => {
    if (!id) return;
    let alive = true;
    const poll = async () => {
      try {
        const r = await getAppResource(id);
        if (alive) setRes(r);
      } catch {}
    };
    poll();
    const iv = setInterval(poll, 2500);
    return () => { alive = false; clearInterval(iv); };
  }, [id]);

  const runScan = async () => {
    if (!id) return;
    setLeftLoading(true); setErr(null);
    try {
      const l = await analyzeLeftovers(id);
      setLeftovers(l);
      if (l.length===0) setErr("No leftovers found — clean. Scanned whole device: all drives (Program Files, Users, AppData, ProgramData, Windows), registry (HKLM/HKCU Software, Run, Services, Uninstall), temp/junk, services, scheduled tasks, shortcuts, modules.");
    } catch(e){ setErr(String(e)); }
    finally{ setLeftLoading(false); }
  };

  useEffect(() => {
    if (!d || !id) return;
    const t = setTimeout(runScan, 400);
    return () => clearTimeout(t);
  }, [d?.id]);

  const categories = useMemo(() => {
    if (!leftovers) return [];
    const counts = new Map<string, number>();
    for (const l of leftovers) {
      const cat = artifactCategory(l.artifact_type, (l as any).description);
      counts.set(cat, (counts.get(cat) ?? 0) + 1);
    }
    return Array.from(counts.entries()).sort((a,b)=> b[1]-a[1]);
  }, [leftovers]);

  const filteredLeftovers = useMemo(() => {
    if (!leftovers) return null;
    if (activeCat === "All") return leftovers;
    return leftovers.filter(l => artifactCategory(l.artifact_type, (l as any).description) === activeCat);
  }, [leftovers, activeCat]);

  const grouped = useMemo(() => {
    if (!filteredLeftovers) return null;
    const isFs = (t:string, desc?: string) => {
      // Duplicates are files but we want them grouped as Duplicates, not fs drive grouping? Keep drive grouping for duplicates too
      if (desc && desc.toLowerCase().includes("duplicate")) return true;
      return ["Directory","File","TempFile"].includes(t);
    };
    const fs = filteredLeftovers.filter(l => isFs(l.artifact_type, (l as any).description));
    const nonFs = filteredLeftovers.filter(l => !isFs(l.artifact_type, (l as any).description));
    const map = new Map<string, LeftoverDto[]>();
    for (const l of fs) {
      const cat = artifactCategory(l.artifact_type, (l as any).description);
      if (cat === "Duplicates") {
        const key = `● Duplicates — safe to delete`;
        if (!map.has(key)) map.set(key, []);
        map.get(key)!.push(l);
        continue;
      }
      const { label, drive } = categorizePath(l.path);
      const key = `${drive} · ${label}`;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(l);
    }
    for (const l of nonFs) {
      const cat = artifactCategory(l.artifact_type, (l as any).description);
      const key = `● ${cat}`;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(l);
    }
    return Array.from(map.entries()).sort((a,b)=> a[0].localeCompare(b[0]));
  }, [filteredLeftovers]);

  const totalBytes = useMemo(() => {
    if (!leftovers) return 0;
    return leftovers.reduce((acc, l) => acc + (l.size_bytes ?? 0), 0);
  }, [leftovers]);

  const totalSizeDisplay = useMemo(() => {
    if (!leftovers || leftovers.length===0) return null;
    if (totalBytes === 0) return `${leftovers.length} items`;
    const mib = totalBytes / 1024 / 1024;
    if (mib >= 1024) return `${(mib/1024).toFixed(2)} GiB total`;
    if (mib >= 1) return `${mib.toFixed(1)} MiB total`;
    return `${(totalBytes/1024).toFixed(0)} KiB total`;
  }, [leftovers, totalBytes]);

  if (!id) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/75 backdrop-blur-md overlay-enter" onClick={onClose} aria-hidden />
      <div role="dialog" aria-modal="true" aria-label="Application Info" className="relative w-full max-w-[880px] max-h-[92vh] rounded-[20px] border border-[rgba(225,29,72,0.16)] bg-[#141414] shadow-[0_24px_80px_rgba(0,0,0,0.75),0_0_50px_rgba(225,29,72,0.08)] overflow-hidden flex flex-col modal-enter">
        <div className="px-6 md:px-8 py-5 border-b border-[rgba(225,29,72,0.08)] flex items-center justify-between bg-[#0A0A0A] shrink-0">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-[8px] bg-[rgba(225,29,72,0.12)] border border-[rgba(225,29,72,0.18)] flex items-center justify-center text-[#E11D48]">
              <ShieldAlert size={14} />
            </div>
            <h3 className="text-[13px] font-semibold tracking-[0.12em] uppercase text-[#F5F0EB]">Application Info</h3>
          </div>
          <button onClick={onClose} aria-label="Close" className="w-8 h-8 rounded-full bg-[#1A1A1A] border border-[rgba(255,255,255,0.06)] flex items-center justify-center text-[#A8A39E] hover:text-[#F5F0EB] hover:border-[rgba(225,29,72,0.18)] transition-colors"><X size={16} /></button>
        </div>

        <div className="flex-1 overflow-auto p-6 md:p-8 space-y-6">
          {!d ? <p className="text-sm text-[#A8A39E] flex items-center gap-2"><Loader2 size={14} className="animate-spin"/> {err ?? "Loading…"}</p> : (
            <>
              <div className="flex gap-5 items-start">
                {iconB64 ? (
                  <img src={`data:image/png;base64,${iconB64}`} alt="" className="w-20 h-20 md:w-24 md:h-24 rounded-[16px] object-contain border border-[rgba(225,29,72,0.12)] bg-black shrink-0" style={{ imageRendering: "auto" } as React.CSSProperties} />
                ) : d.icon_color ? (
                  <div className="w-20 h-20 md:w-24 md:h-24 rounded-[16px] flex items-center justify-center font-bold text-white text-2xl border border-[rgba(225,29,72,0.12)] shrink-0" style={{ backgroundColor: `rgb(${d.icon_color})` }}>{d.name.slice(0,2).toUpperCase()}</div>
                ) : (
                  <div className="w-20 h-20 md:w-24 md:h-24 rounded-[16px] bg-[#E11D48] text-white flex items-center justify-center font-bold text-2xl shrink-0">{d.name.slice(0,2).toUpperCase()}</div>
                )}
                <div className="min-w-0 flex-1">
                  <h4 className="text-lg md:text-xl font-semibold text-[#F5F0EB] leading-tight break-words">{d.name}</h4>
                  <p className="text-sm text-[#A8A39E] mt-1 break-words">{d.publisher ?? "Unknown publisher"} · {d.source_label}</p>
                  {d.is_system && <span className="mt-3 inline-flex items-center gap-1.5 rounded-full bg-[rgba(255,165,0,0.12)] border border-[rgba(255,165,0,0.25)] px-3 py-1 text-xs font-medium text-[#FF801F]"><ShieldAlert size={12}/> System component</span>}
                </div>
              </div>

              <div className={`rounded-[16px] border p-4 ${res?.is_running ? "bg-[rgba(17,255,153,0.06)] border-[rgba(17,255,153,0.18)]" : "bg-[#1A1A1A] border-[rgba(225,29,72,0.06)]"}`}>
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold tracking-[0.08em] uppercase text-[#A8A39E] inline-flex items-center gap-1.5"><Activity size={14} className={res?.is_running ? "text-[#11FF99]" : "text-[#6B6661]"} /> Resource consumption</span>
                  <span className={`text-xs font-medium px-2.5 py-1 rounded-full border ${res?.is_running ? "bg-[#11FF99] text-black border-[#11FF99]" : "bg-black text-[#6B6661] border-[rgba(225,29,72,0.08)]"}`}>{res?.is_running ? `Running · ${res.process_count} proc` : "Not running"}</span>
                </div>
                {res?.is_running ? (
                  <div className="mt-4 grid grid-cols-3 gap-3 text-xs">
                    <div className="bg-black rounded-[12px] border border-[rgba(225,29,72,0.06)] p-3">
                      <div className="text-[#6B6661] flex items-center gap-1 text-[11px] tracking-[0.08em] uppercase"><Cpu size={12}/> CPU</div>
                      <div className="text-[18px] font-bold text-[#F5F0EB] mt-1">{res.cpu.toFixed(1)}%</div>
                      {res.gpu > 0.5 && <div className="text-[11px] text-[#A8A39E] flex items-center gap-1 mt-1"><Zap size={10}/> GPU {res.gpu.toFixed(1)}%</div>}
                    </div>
                    <div className="bg-black rounded-[12px] border border-[rgba(225,29,72,0.06)] p-3">
                      <div className="text-[#6B6661] flex items-center gap-1 text-[11px] tracking-[0.08em] uppercase"><MemoryStick size={12}/> Memory</div>
                      <div className="text-[15px] font-semibold text-[#F5F0EB] mt-1">{res.memory_display ?? "—"}</div>
                      <div className="text-[11px] text-[#6B6661] mt-1">{res.pid ? `PID ${res.pid}` : ""}</div>
                    </div>
                    <div className="bg-black rounded-[12px] border border-[rgba(225,29,72,0.06)] p-3">
                      <div className="text-[#6B6661] text-[11px] tracking-[0.08em] uppercase">Exe</div>
                      <div className="text-[11px] font-medium text-[#F5F0EB] truncate mt-1" title={res.exe_path ?? ""}>{res.exe_path ? res.exe_path.split("\\").pop() : "—"}</div>
                      <div className="text-[11px] text-[#6B6661] mt-1">{res.vram_bytes ? `VRAM ${(res.vram_bytes/1024/1024).toFixed(0)} MiB` : ""}</div>
                    </div>
                  </div>
                ) : (
                  <p className="mt-3 text-xs text-[#6B6661]">No active process detected. Resources update live every 2.5s.</p>
                )}
              </div>

              <dl className="grid grid-cols-2 md:grid-cols-4 gap-3 text-sm">
                <div className="bg-[#1A1A1A] rounded-[14px] p-4 border border-[rgba(225,29,72,0.06)]"><dt className="text-[11px] tracking-[0.08em] uppercase text-[#6B6661]">Version</dt><dd className="font-medium text-[#F5F0EB] mt-1 break-words">{d.version ?? "—"}</dd></div>
                <div className="bg-[#1A1A1A] rounded-[14px] p-4 border border-[rgba(225,29,72,0.06)]"><dt className="text-[11px] tracking-[0.08em] uppercase text-[#6B6661]">Size</dt><dd className="font-medium text-[#F5F0EB] mt-1">{d.size_display ?? "—"}</dd></div>
                <div className="bg-[#1A1A1A] rounded-[14px] p-4 border border-[rgba(225,29,72,0.06)]"><dt className="text-[11px] tracking-[0.08em] uppercase text-[#6B6661]">Installed</dt><dd className="font-medium text-[#F5F0EB] mt-1">{d.install_date ?? "—"}</dd></div>
                <div className="bg-[#1A1A1A] rounded-[14px] p-4 border border-[rgba(225,29,72,0.06)]"><dt className="text-[11px] tracking-[0.08em] uppercase text-[#6B6661]">Source</dt><dd className="font-medium text-[#F5F0EB] mt-1">{d.source_label}</dd></div>
              </dl>

              <div className="grid md:grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <label className="text-[11px] font-semibold tracking-[0.08em] uppercase text-[#A8A39E]">Install location</label>
                  <div className="flex items-start gap-2 rounded-[12px] border border-[rgba(225,29,72,0.08)] bg-black px-3 py-3 text-xs text-[#A8A39E] break-all min-h-[44px]">
                    <FolderOpen size={14} className="shrink-0 mt-0.5 text-[#6B6661]" /> <span className="break-words">{d.install_location ?? "—"}</span>
                  </div>
                </div>
                <div className="space-y-1.5">
                  <label className="text-[11px] font-semibold tracking-[0.08em] uppercase text-[#A8A39E]">Uninstall string</label>
                  <div className="rounded-[12px] border border-[rgba(225,29,72,0.06)] bg-black px-3 py-3 text-xs text-[#6B6661] break-all font-mono min-h-[44px]">{d.uninstall_string ?? "—"}</div>
                </div>
              </div>
              {d.quiet_uninstall_string && <div className="rounded-[12px] border border-[rgba(225,29,72,0.06)] bg-black px-3 py-3 text-xs text-[#6B6661] break-all font-mono"><span className="font-semibold text-[#A8A39E]">Quiet:</span> {d.quiet_uninstall_string}</div>}
              {d.registry_keys.length>0 && <div className="rounded-[12px] border border-[rgba(225,29,72,0.06)] bg-black px-4 py-3 text-xs text-[#6B6661] font-mono"><p className="font-semibold text-[#A8A39E] text-[11px] tracking-[0.08em] uppercase mb-1">Registry</p>{d.registry_keys.map(k=> <div key={k} className="truncate py-0.5">{k}</div>)}</div>}

              {/* Comprehensive leftovers — whole device: folders, junk, modules, files, registry, services, tasks, shortcuts */}
              <div className="rounded-[16px] border border-[rgba(225,29,72,0.14)] overflow-hidden bg-[#0A0A0A]">
                <div className="px-4 py-3 bg-[#1A1A1A] border-b border-[rgba(225,29,72,0.08)] flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2 text-xs font-semibold tracking-[0.08em] uppercase text-[#A8A39E]">
                    <HardDrive size={14} className="text-[#E11D48]" /> Leftovers — whole device
                    <span className="ml-1 text-[#6B6661] normal-case tracking-normal font-medium">
                      {leftLoading ? "Scanning…" : leftovers ? `• ${leftovers.length} items • ${totalSizeDisplay ?? ""}` : "• auto-scanning"}
                    </span>
                  </div>
                  <button disabled={leftLoading} onClick={runScan} className="inline-flex items-center gap-1.5 rounded-full border border-[rgba(225,29,72,0.18)] bg-black px-3 py-1.5 text-[11px] font-medium text-[#F5F0EB] hover:bg-[#1A1A1A] disabled:opacity-50">
                    {leftLoading ? <Loader2 size={12} className="animate-spin"/> : <Search size={12}/>} {leftLoading ? "Scanning…" : "Rescan whole device"}
                  </button>
                </div>

                <div className="px-4 py-2.5 bg-black/50 border-b border-[rgba(225,29,72,0.06)] text-[11px] text-[#6B6661] leading-relaxed">
                  Scans <span className="text-[#A8A39E]">every drive</span> (Program Files, Users\*\AppData, ProgramData, Windows shallow, <span className="text-[#11FF99]">all Downloads/Desktop</span> for duplicate installers) + <span className="text-[#A8A39E]">registry</span> (HKLM/HKCU Software, Run, Services, Uninstall, App Paths, Classes) + <span className="text-[#A8A39E]">junk/temp</span> (Windows\Temp, %TEMP%, Prefetch, cache) + <span className="text-[#A8A39E]">services</span>, <span className="text-[#A8A39E]">scheduled tasks</span>, <span className="text-[#A8A39E]">shortcuts</span>, <span className="text-[#A8A39E]">modules</span> & <span className="text-[#11FF99]">duplicate downloads</span> (e.g. Cursor-Setup.exe copies in Downloads/D:). Duplicates are <span className="text-[#11FF99]">Safe</span> — deleting them does NOT affect the installed app.
                </div>

                {/* Category chips */}
                {!leftLoading && leftovers && leftovers.length>0 && (
                  <div className="px-3 py-2.5 bg-[#0F0F0F] border-b border-[rgba(225,29,72,0.06)] flex flex-wrap gap-1.5">
                    <button onClick={()=>setActiveCat("All")} className={`px-3 py-1 rounded-full text-[11px] font-medium border ${activeCat==="All" ? "bg-[#E11D48] text-white border-[#E11D48]" : "bg-black text-[#A8A39E] border-[rgba(255,255,255,0.08)] hover:border-[rgba(225,29,72,0.18)]"}`}>All • {leftovers.length}</button>
                    {categories.map(([cat, count])=> (
                      <button key={cat} onClick={()=>setActiveCat(cat)} className={`inline-flex items-center gap-1 px-3 py-1 rounded-full text-[11px] font-medium border ${activeCat===cat ? "bg-[#E11D48] text-white border-[#E11D48]" : "bg-black text-[#A8A39E] border-[rgba(255,255,255,0.08)] hover:border-[rgba(225,29,72,0.18)]"}`}>{categoryIcon(cat)} {cat} • {count}</button>
                    ))}
                  </div>
                )}

                {leftLoading && (
                  <div className="p-8 flex flex-col items-center gap-3 text-[#A8A39E]">
                    <Loader2 size={20} className="animate-spin text-[#E11D48]" />
                    <p className="text-xs">Scanning whole device for “{d.name}” — folders, junk, modules, files, registry…</p>
                    <p className="text-[11px] text-[#6B6661]">Checks all drives + registry hives + temp + services + tasks + shortcuts. Accurate sizes calculated.</p>
                  </div>
                )}

                {!leftLoading && leftovers && leftovers.length===0 && (
                  <p className="p-6 text-xs text-[#6B6661] text-center">No leftovers found — clean. Checked whole device: drives, registry, temp, services, tasks, shortcuts.</p>
                )}

                {!leftLoading && leftovers && leftovers.length>0 && grouped && filteredLeftovers && (
                  <div className="divide-y divide-[rgba(225,29,72,0.06)] max-h-[420px] overflow-auto">
                    <div className="px-4 py-3 flex items-center justify-between bg-[rgba(225,29,72,0.06)] text-xs">
                      <span className="text-[#A8A39E] inline-flex items-center gap-1.5"><Folder size={12}/> {filteredLeftovers.length} {activeCat==="All" ? "items" : activeCat.toLowerCase()} {activeCat!=="All" ? `• ${activeCat}` : "• folders + junk + registry + services + tasks + shortcuts"} </span>
                      <span className="text-[#F5F0EB] font-medium">{totalSizeDisplay ?? ""}</span>
                    </div>
                    {grouped.map(([group, items]) => (
                      <div key={group} className="">
                        <div className="px-4 py-2 bg-[#141414] text-[11px] font-semibold tracking-[0.06em] uppercase text-[#6B6661] flex items-center gap-2">
                          {group.startsWith("●") ? <Database size={12} className="text-[#E11D48]"/> : <HardDrive size={12} className="text-[#6B6661]"/>} {group} <span className="text-[#3A3937]">•</span> <span className="normal-case tracking-normal font-medium text-[#A8A39E]">{items.length} item{items.length!==1?"s":""}</span>
                        </div>
                        <ul className="divide-y divide-[rgba(255,255,255,0.04)]">
                          {items.map(l=> {
                            const isDuplicate = !!(l as any).description && String((l as any).description).toLowerCase().includes("duplicate");
                            const isDir = l.artifact_type === "Directory";
                            const isReg = l.artifact_type.toLowerCase().includes("registry");
                            const isService = l.artifact_type === "Service";
                            const isTask = l.artifact_type === "ScheduledTask";
                            const isShortcut = l.artifact_type === "Shortcut";
                            return (
                              <li key={l.id} className={`px-4 py-3 flex items-start gap-3 hover:bg-[rgba(255,255,255,0.02)] ${isDuplicate ? "bg-[rgba(17,255,153,0.04)]" : ""}`}>
                                <div className={`mt-0.5 w-7 h-7 rounded-[8px] bg-black border flex items-center justify-center shrink-0 ${isDuplicate ? "border-[rgba(17,255,153,0.25)]" : "border-[rgba(225,29,72,0.08)]"}`}>
                                  {isDuplicate ? <Copy size={12} className="text-[#11FF99]"/> : isReg ? <Database size={12} className="text-[#9B8CFF]"/> : isService ? <Layers size={12} className="text-[#FF801F]"/> : isTask ? <Clock3 size={12} className="text-[#FFC107]"/> : isShortcut ? <Link2 size={12} className="text-[#11FF99]"/> : isDir ? <Folder size={12} className="text-[#E11D48]"/> : <File size={12} className="text-[#6B6661]"/>}
                                </div>
                                <div className="min-w-0 flex-1">
                                  <p className="text-xs font-medium text-[#F5F0EB] break-all leading-relaxed" title={l.path}>{l.path}</p>
                                  {isDuplicate && <p className="text-[11px] text-[#11FF99] mt-1 flex items-center gap-1"><CheckCircle2 size={10}/> Duplicate installer/download — <b>safe to delete</b>, does NOT affect installed app at {d.install_location ?? "install location"}</p>}
                                  <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px]">
                                    <span className={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 font-medium ${safetyBadge(l.safety)}`}>
                                      {l.safety === "Safe" ? <CheckCircle2 size={10}/> : l.safety === "Critical" ? <AlertTriangle size={10}/> : <ShieldAlert size={10}/>} {l.safety}
                                    </span>
                                    <span className="text-[#6B6661] inline-flex items-center gap-1">{categoryIcon(artifactCategory(l.artifact_type, (l as any).description))} {l.artifact_type}{isDuplicate ? " • Duplicate" : ""}</span>
                                    <span className="text-[#3A3937]">•</span>
                                    <span className="text-[#A8A39E]">confidence {(l.confidence*100).toFixed(0)}%</span>
                                  </div>
                                  {l.artifact_type.toLowerCase().includes("registry") && <p className="text-[11px] text-[#6B6661] mt-1">Registry leftover — blocked if under protected hive</p>}
                                </div>
                                <div className="shrink-0 text-right">
                                  <div className="text-xs font-semibold text-[#F5F0EB]">{l.size_display ?? (isReg || isService || isTask ? "—" : "0 B")}</div>
                                  <div className="text-[11px] text-[#6B6661]">size</div>
                                </div>
                              </li>
                            );
                          })}
                        </ul>
                      </div>
                    ))}
                  </div>
                )}

                {!leftLoading && !leftovers && (
                  <p className="p-6 text-xs text-[#6B6661] text-center">Opening… will auto-scan whole device.</p>
                )}
              </div>

              {err && <p className="text-xs text-[#FF8DA0] bg-[rgba(255,32,71,0.08)] border border-[rgba(255,32,71,0.15)] rounded-[10px] px-3 py-2">{err}</p>}

              {showDeleteConfirm && leftovers && leftovers.length>0 && (
                <div className="rounded-[14px] border border-[rgba(255,193,7,0.25)] bg-[rgba(255,193,7,0.06)] p-4">
                  <p className="text-xs font-semibold text-[#FFC107] flex items-center gap-1.5"><AlertTriangle size={14}/> Confirm delete</p>
                  <p className="text-xs text-[#A8A39E] mt-1">The app will be uninstalled and <span className="text-[#F5F0EB] font-medium">{leftovers.length} leftover item{leftovers.length!==1?"s":""} ({categories.map(([c,n])=>`${c}:${n}`).join(" • ")})</span> will be removed (where safe). Protected Windows/registry paths are blocked.</p>
                  <div className="mt-3 flex gap-2">
                    <button onClick={()=>setShowDeleteConfirm(false)} className="rounded-full bg-black border border-[rgba(255,255,255,0.08)] px-4 py-2 text-xs text-[#A8A39E]">Cancel</button>
                    <button
                      onClick={async ()=>{
                        if (!d) return;
                        setBusy("uninstall");
                        try { await uninstallApplications({ ids: [d.id], force: true }); onUninstalled(); onClose(); } catch(e){ setErr(String(e)); } finally{ setBusy(null); setShowDeleteConfirm(false); }
                      }}
                      className="rounded-full bg-[#E11D48] px-4 py-2 text-xs font-semibold text-white"
                    >Confirm & Uninstall</button>
                  </div>
                </div>
              )}
            </>
          )}
        </div>

        {d && (
          <div className="px-6 md:px-8 py-4 border-t border-[rgba(225,29,72,0.08)] bg-[#0A0A0A] flex flex-col sm:flex-row gap-3 shrink-0">
            <button
              disabled={!!busy}
              onClick={async () => {
                if (!leftovers && !leftLoading) { await runScan(); }
                setShowDeleteConfirm(true);
                setTimeout(()=>{ document.querySelector('[role="dialog"]')?.scrollTo({ top: 9999, behavior: 'smooth'}); }, 100);
              }}
              className="flex-1 inline-flex items-center justify-center gap-2 rounded-full bg-[#F5F0EB] px-5 py-3 text-[13px] font-semibold text-black hover:bg-white disabled:opacity-50 transition"
            ><Trash2 size={16}/>{busy==="uninstall" ? "Working…" : "Delete — show leftovers"}</button>
            <button
              disabled={!!busy}
              onClick={async () => {
                setBusy("force");
                try { await uninstallApplications({ ids: [d.id], force:true }); onUninstalled(); onClose(); } catch(e){ setErr(String(e)); } finally{ setBusy(null); }
              }}
              className="flex-1 inline-flex items-center justify-center gap-2 rounded-full bg-[#E11D48] px-5 py-3 text-[13px] font-semibold text-white hover:bg-[#FF3B6A] disabled:opacity-50 transition shadow-[0_0_20px_rgba(225,29,72,0.25)]"
            ><Flame size={16}/>{busy==="force" ? "Force removing…" : "Force Remove"}</button>
            <button
              disabled={!!busy || leftLoading}
              onClick={runScan}
              className="hidden sm:inline-flex items-center justify-center gap-2 rounded-full border border-[rgba(225,29,72,0.12)] bg-[#1A1A1A] px-5 py-3 text-[13px] font-medium text-[#F5F0EB] hover:bg-black hover:border-[rgba(225,29,72,0.2)] transition"
            ><Search size={16}/>{leftLoading ? "Scanning…" : "Rescan"}</button>
          </div>
        )}
      </div>
    </div>
  );
}
