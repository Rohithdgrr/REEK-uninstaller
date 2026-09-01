import { useEffect, useState } from "react";
import { X, FolderOpen, Trash2, Flame, Search, ShieldAlert, Cpu, MemoryStick, Activity, Zap } from "lucide-react";
import { getAppDetails, analyzeLeftovers, type AppDetails, type LeftoverDto, uninstallApplications, getAppIcon, getAppResource, type AppResourceDto } from "../lib/tauri";

export function AppDetailsDrawer({
  id, onClose, onUninstalled,
}: { id: string | null; onClose: () => void; onUninstalled: () => void; }) {
  const [d, setD] = useState<AppDetails | null>(null);
  const [leftovers, setLeftovers] = useState<LeftoverDto[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [iconB64, setIconB64] = useState<string | null>(null);
  const [res, setRes] = useState<AppResourceDto | null>(null);

  useEffect(() => {
    if (!id) return;
    setD(null); setLeftovers(null); setErr(null); setIconB64(null); setRes(null);
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

  if (!id) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/75 backdrop-blur-md overlay-enter" onClick={onClose} aria-hidden />
      <div role="dialog" aria-modal="true" aria-label="Application Info" className="relative w-full max-w-[760px] max-h-[90vh] rounded-[20px] border border-[rgba(225,29,72,0.16)] bg-[#141414] shadow-[0_24px_80px_rgba(0,0,0,0.75),0_0_50px_rgba(225,29,72,0.08)] overflow-hidden flex flex-col modal-enter">
        {/* Header */}
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
          {!d ? <p className="text-sm text-[#A8A39E]">{err ?? "Loading…"}</p> : (
            <>
              {/* Hero — icon + name side-by-side for large dialog */}
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

              {/* Resource consumption */}
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

              {leftovers && (
                <div className="rounded-[14px] border border-[rgba(225,29,72,0.12)] overflow-hidden bg-[#0A0A0A]">
                  <div className="px-4 py-3 bg-[#1A1A1A] border-b border-[rgba(225,29,72,0.06)] text-xs font-semibold tracking-[0.08em] uppercase text-[#A8A39E]">Leftovers ({leftovers.length})</div>
                  {leftovers.length===0 ? <p className="p-6 text-xs text-[#6B6661] text-center">No artifacts — clean.</p> : (
                    <ul className="divide-y divide-[rgba(225,29,72,0.06)] max-h-56 overflow-auto">
                      {leftovers.map(l=> (
                        <li key={l.id} className="px-4 py-3 flex justify-between gap-4">
                          <p className="text-xs font-medium text-[#F5F0EB] truncate flex-1">{l.path}</p>
                          <span className="text-[11px] text-[#6B6661] shrink-0">{l.artifact_type} · {l.safety}</span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              )}
              {err && <p className="text-xs text-[#FF8DA0] bg-[rgba(255,32,71,0.08)] border border-[rgba(255,32,71,0.15)] rounded-[10px] px-3 py-2">{err}</p>}
            </>
          )}
        </div>

        {d && (
          <div className="px-6 md:px-8 py-4 border-t border-[rgba(225,29,72,0.08)] bg-[#0A0A0A] flex flex-col sm:flex-row gap-3 shrink-0">
            <button
              disabled={!!busy}
              onClick={async () => {
                setBusy("uninstall");
                try { await uninstallApplications({ ids: [d.id], force:false }); onUninstalled(); onClose(); } catch(e){ setErr(String(e)); } finally{ setBusy(null); }
              }}
              className="flex-1 inline-flex items-center justify-center gap-2 rounded-full bg-[#F5F0EB] px-5 py-3 text-[13px] font-semibold text-black hover:bg-white disabled:opacity-50 transition"
            ><Trash2 size={16}/>{busy==="uninstall" ? "Working…" : "Uninstall"}</button>
            <button
              disabled={!!busy}
              onClick={async () => {
                setBusy("force");
                try { await uninstallApplications({ ids: [d.id], force:true }); onUninstalled(); onClose(); } catch(e){ setErr(String(e)); } finally{ setBusy(null); }
              }}
              className="flex-1 inline-flex items-center justify-center gap-2 rounded-full bg-[#E11D48] px-5 py-3 text-[13px] font-semibold text-white hover:bg-[#FF3B6A] disabled:opacity-50 transition shadow-[0_0_20px_rgba(225,29,72,0.25)]"
            ><Flame size={16}/>{busy==="force" ? "Force removing…" : "Force Remove"}</button>
            <button
              disabled={!!busy}
              onClick={async () => {
                setBusy("left");
                try { const l = await analyzeLeftovers(d.id); setLeftovers(l); if (l.length===0) setErr("No leftovers found — clean."); } catch(e){ setErr(String(e)); } finally{ setBusy(null); }
              }}
              className="flex-1 inline-flex items-center justify-center gap-2 rounded-full border border-[rgba(225,29,72,0.12)] bg-[#1A1A1A] px-5 py-3 text-[13px] font-medium text-[#F5F0EB] hover:bg-black hover:border-[rgba(225,29,72,0.2)] transition"
            ><Search size={16}/>{busy==="left" ? "Scanning…" : "Scan Leftovers"}</button>
          </div>
        )}
      </div>
    </div>
  );
}
