import { useEffect, useState } from "react";
import { X, FolderOpen, Trash2, Flame, Search, ShieldAlert } from "lucide-react";
import { getAppDetails, analyzeLeftovers, type AppDetails, type LeftoverDto, uninstallApplications } from "../lib/tauri";

export function AppDetailsDrawer({
  id, onClose, onUninstalled,
}: { id: string | null; onClose: () => void; onUninstalled: () => void; }) {
  const [d, setD] = useState<AppDetails | null>(null);
  const [leftovers, setLeftovers] = useState<LeftoverDto[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    setD(null); setLeftovers(null); setErr(null);
    getAppDetails(id).then(setD).catch(e => setErr(String(e)));
  }, [id]);

  if (!id) return null;
  return (
    <div className="fixed inset-0 z-40 flex justify-end">
      <div className="absolute inset-0 bg-slate-900/30 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-md bg-white border-l border-slate-200 shadow-xl h-full flex flex-col">
        <div className="px-6 py-4 border-b border-slate-200 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-slate-900">Application Info</h3>
          <button onClick={onClose} aria-label="Close" className="p-1.5 rounded-lg hover:bg-slate-100"><X size={16} /></button>
        </div>
        <div className="flex-1 overflow-auto p-6 space-y-5">
          {!d ? <p className="text-sm text-slate-500">{err ?? "Loading…"}</p> : (
            <>
              <div>
                <div className="w-12 h-12 rounded-xl bg-blue-600 text-white flex items-center justify-center font-bold">{d.name.slice(0,2).toUpperCase()}</div>
                <h4 className="mt-3 text-lg font-semibold text-slate-900">{d.name}</h4>
                <p className="text-sm text-slate-500">{d.publisher ?? "Unknown publisher"} · {d.source_label}</p>
                {d.is_system && <span className="mt-2 inline-flex items-center gap-1 rounded-full bg-amber-50 border border-amber-200 px-2.5 py-1 text-xs text-amber-700"><ShieldAlert size={12}/> System component</span>}
              </div>

              <dl className="grid grid-cols-2 gap-3 text-sm">
                <div className="bg-slate-50 rounded-xl p-3 border border-slate-200"><dt className="text-xs text-slate-500">Version</dt><dd className="font-medium text-slate-900">{d.version ?? "—"}</dd></div>
                <div className="bg-slate-50 rounded-xl p-3 border border-slate-200"><dt className="text-xs text-slate-500">Size</dt><dd className="font-medium text-slate-900">{d.size_display ?? "—"}</dd></div>
                <div className="bg-slate-50 rounded-xl p-3 border border-slate-200"><dt className="text-xs text-slate-500">Installed</dt><dd className="font-medium text-slate-900">{d.install_date ?? "—"}</dd></div>
                <div className="bg-slate-50 rounded-xl p-3 border border-slate-200"><dt className="text-xs text-slate-500">Source</dt><dd className="font-medium text-slate-900">{d.source_label}</dd></div>
              </dl>

              <div className="space-y-2">
                <label className="text-xs font-semibold text-slate-700">Install location</label>
                <div className="flex items-center gap-2 rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-xs text-slate-700 break-all">
                  <FolderOpen size={14} className="shrink-0 text-slate-500" /> {d.install_location ?? "—"}
                </div>
                <label className="text-xs font-semibold text-slate-700">Uninstall string</label>
                <div className="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-xs text-slate-600 break-all">{d.uninstall_string ?? "—"}</div>
                {d.quiet_uninstall_string && <div className="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-xs text-slate-600 break-all"><span className="font-semibold">Quiet:</span> {d.quiet_uninstall_string}</div>}
                {d.registry_keys.length>0 && <div className="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-xs text-slate-600"><p className="font-semibold text-slate-700">Registry</p>{d.registry_keys.map(k=> <div key={k} className="truncate">{k}</div>)}</div>}
              </div>

              <div className="flex flex-col gap-2">
                <button
                  disabled={!!busy}
                  onClick={async () => {
                    setBusy("uninstall");
                    try { await uninstallApplications({ ids: [d.id], force:false }); onUninstalled(); onClose(); } catch(e){ setErr(String(e)); } finally{ setBusy(null); }
                  }}
                  className="inline-flex items-center justify-center gap-2 rounded-xl bg-blue-600 px-4 py-2.5 text-sm font-semibold text-white hover:bg-blue-700 disabled:opacity-50"
                ><Trash2 size={16}/>{busy==="uninstall" ? "Working…" : "Uninstall"}</button>
                <button
                  disabled={!!busy}
                  onClick={async () => {
                    setBusy("force");
                    try { await uninstallApplications({ ids: [d.id], force:true }); onUninstalled(); onClose(); } catch(e){ setErr(String(e)); } finally{ setBusy(null); }
                  }}
                  className="inline-flex items-center justify-center gap-2 rounded-xl bg-red-600 px-4 py-2.5 text-sm font-semibold text-white hover:bg-red-700 disabled:opacity-50"
                ><Flame size={16}/>{busy==="force" ? "Force removing…" : "Force Remove"}</button>
                <button
                  disabled={!!busy}
                  onClick={async () => {
                    setBusy("left");
                    try { const l = await analyzeLeftovers(d.id); setLeftovers(l); if (l.length===0) setErr("No leftovers found — clean."); } catch(e){ setErr(String(e)); } finally{ setBusy(null); }
                  }}
                  className="inline-flex items-center justify-center gap-2 rounded-xl border border-slate-200 bg-white px-4 py-2.5 text-sm font-medium text-slate-700 hover:bg-slate-50"
                ><Search size={16}/>{busy==="left" ? "Scanning…" : "Scan Leftovers"}</button>
              </div>

              {leftovers && (
                <div className="rounded-xl border border-slate-200 overflow-hidden">
                  <div className="px-4 py-2 bg-slate-50 text-xs font-semibold text-slate-700">Leftovers ({leftovers.length})</div>
                  {leftovers.length===0 ? <p className="p-4 text-xs text-slate-500">No artifacts.</p> : (
                    <ul className="divide-y divide-slate-100 max-h-48 overflow-auto">
                      {leftovers.map(l=> (
                        <li key={l.id} className="px-4 py-2">
                          <p className="text-xs font-medium text-slate-900 truncate">{l.path}</p>
                          <p className="text-[11px] text-slate-500">{l.artifact_type} · {l.safety} · conf {(l.confidence*100).toFixed(0)}% {l.size_display ?? ""}</p>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              )}
              {err && <p className="text-xs text-red-600">{err}</p>}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
