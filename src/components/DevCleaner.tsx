import { useEffect, useState, useMemo } from "react";
import { Package, HardDrive, Trash2, Loader2, Search, Folder, FileStack, Layers, CheckSquare, Square, Zap } from "lucide-react";
import { scanDevModules, cleanDevModules, cleanAllDevModules, type DevModuleDto } from "../lib/tauri";

const kindMeta: Record<string, { label: string; color: string }> = {
  "node_modules": { label: "Node", color: "#68A063" },
  "python-venv": { label: "Python venv", color: "#3776AB" },
  "python-cache": { label: "Python cache", color: "#FFD343" },
  "rust-target": { label: "Rust target", color: "#DEA584" },
  "java-target": { label: "Java target", color: "#ED8B00" },
  "gradle-cache": { label: "Gradle", color: "#02303A" },
  "next-build": { label: "Next/Nuxt", color: "#000000" },
  "dist": { label: "dist", color: "#6B7280" },
  "build": { label: "build", color: "#6B7280" },
  "vendor": { label: "vendor", color: "#4F5D95" },
};

export function DevCleaner({ onCleaned }: { onCleaned?: (count: number, bytes: number) => void }) {
  const [modules, setModules] = useState<DevModuleDto[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [cleaning, setCleaning] = useState(false);
  const [filterLang, setFilterLang] = useState<string>("All");

  const load = async () => {
    setLoading(true);
    try {
      const m = await scanDevModules();
      setModules(m);
    } catch (e) { console.error(e); setModules([]); }
    finally { setLoading(false); }
  };
  useEffect(()=>{ load(); }, []);

  const languages = useMemo(()=> {
    if (!modules) return [];
    const s = new Set(modules.map(m=>m.language));
    return ["All", ...Array.from(s)];
  }, [modules]);

  const filtered = useMemo(()=>{
    if (!modules) return [];
    let out = modules;
    if (filterLang !== "All") out = out.filter(m=> m.language===filterLang);
    if (query.trim()) {
      const q = query.toLowerCase();
      out = out.filter(m=> m.path.toLowerCase().includes(q) || m.name.toLowerCase().includes(q) || m.kind.toLowerCase().includes(q));
    }
    return out;
  }, [modules, query, filterLang]);

  const totalBytes = useMemo(()=> filtered.reduce((a,m)=>a+m.size_bytes,0), [filtered]);
  const totalDisplay = totalBytes>0 ? (totalBytes/1024/1024/1024>=1 ? `${(totalBytes/1024/1024/1024).toFixed(2)} GiB` : `${(totalBytes/1024/1024).toFixed(1)} MiB`) : "—";
  const selectedBytes = useMemo(()=> {
    if (!modules) return 0;
    return modules.filter(m=> selected.has(m.path)).reduce((a,m)=>a+m.size_bytes,0);
  }, [modules, selected]);

  const grouped = useMemo(()=>{
    const map = new Map<string, DevModuleDto[]>();
    for (const m of filtered) {
      const key = m.language || m.kind;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(m);
    }
    return Array.from(map.entries()).sort((a,b)=> b[1].reduce((s,x)=>s+x.size_bytes,0) - a[1].reduce((s,x)=>s+x.size_bytes,0));
  }, [filtered]);

  const toggle = (path:string) => setSelected(s=>{ const n=new Set(s); if(n.has(path)) n.delete(path); else n.add(path); return n; });
  const toggleAll = () => {
    if (selected.size===filtered.length) setSelected(new Set());
    else setSelected(new Set(filtered.map(m=>m.path)));
  };

  const handleDeleteSelected = async () => {
    if (selected.size===0) return;
    setCleaning(true);
    try {
      const paths = Array.from(selected);
      await cleanDevModules(paths);
      setModules(prev => prev ? prev.filter(m=> !selected.has(m.path)) : prev);
      const bytes = selectedBytes;
      const count = selected.size;
      setSelected(new Set());
      onCleaned?.(count, bytes);
    } catch(e){ console.error(e); } finally{ setCleaning(false); }
  };
  const handleDeleteAll = async () => {
    if (!modules || modules.length===0) return;
    if (!confirm(`Delete ALL ${filtered.length} dev folders (${totalDisplay})? This will delete node_modules, venv, target, dist etc. They can be reinstalled via npm/pip/cargo.`)) return;
    setCleaning(true);
    try {
      const paths = filtered.map(m=>m.path);
      await cleanDevModules(paths);
      setModules(prev => prev ? prev.filter(m=> !paths.includes(m.path)) : prev);
      setSelected(new Set());
      onCleaned?.(paths.length, totalBytes);
    } catch(e){ console.error(e); } finally{ setCleaning(false); }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-xl bg-[#0A0A0A] border border-[rgba(201,168,76,0.18)] flex items-center justify-center text-[#C9A84C]"><Package size={18}/></div>
          <div>
            <h3 className="text-[16px] font-semibold text-white">Dev Cleaner — One-Click Purge</h3>
            <p className="text-[11px] text-[#6B6661]">{loading ? "Scanning whole device…" : `${filtered.length} modules • ${totalDisplay} reclaimable`}</p>
          </div>
        </div>
        <button onClick={load} disabled={loading} className="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-[#141414] px-3 py-1.5 text-xs text-[#A8A39E] hover:text-white disabled:opacity-50">
          {loading ? <Loader2 size={12} className="animate-spin"/> : <Search size={12}/>} {loading ? "Scanning…" : "Rescan"}
        </button>
      </div>

      <div className="rounded-xl bg-[rgba(201,168,76,0.08)] border border-[rgba(201,168,76,0.18)] px-4 py-3 flex items-start gap-2">
        <Zap size={14} className="text-[#C9A84C] mt-0.5 shrink-0"/>
        <p className="text-xs text-[#C9A84C] leading-relaxed">
          Finds <b>node_modules</b>, <b>Python venv</b> (.venv/venv/__pycache__), <b>Rust target</b>, <b>Java target/.gradle</b>, <b>dist/build/out/.next</b>, <b>vendor</b> etc. across all drives & all users (Documents, Projects, code, Desktop). One tap deletes <b>all</b> — safe, recreatable via <code className="bg-black/30 px-1 rounded">npm install / pip install / cargo build</code>.
        </p>
      </div>

      <div className="flex flex-wrap gap-2">
        <div className="flex-1 min-w-[180px] relative">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#6B6661]"/>
          <input value={query} onChange={e=>setQuery(e.target.value)} placeholder="Filter by path, name, kind…" className="w-full bg-[#0A0A0A] border border-white/10 rounded-full pl-9 pr-4 py-2.5 text-sm text-white placeholder:text-[#6B6661] focus:outline-none focus:border-[rgba(201,168,76,0.3)]"/>
        </div>
        <div className="flex gap-1.5 overflow-auto">
          {languages.map(lang=> (
            <button key={lang} onClick={()=>setFilterLang(lang)} className={`px-3 py-2 rounded-full text-xs font-medium border whitespace-nowrap ${filterLang===lang ? "bg-[#C9A84C] text-black border-[#C9A84C]" : "bg-[#141414] text-[#A8A39E] border-white/10 hover:text-white"}`}>{lang}</button>
          ))}
        </div>
      </div>

      {filtered.length>0 && (
        <div className="flex items-center justify-between bg-[#141414] border border-white/5 rounded-xl px-4 py-3">
          <label className="flex items-center gap-2 text-sm text-white cursor-pointer" onClick={toggleAll}>
            <span className={`w-5 h-5 rounded-md border flex items-center justify-center ${selected.size===filtered.length && filtered.length>0 ? "bg-[#C9A84C] border-[#C9A84C] text-black" : "border-white/15 bg-black"}`}>
              {selected.size===filtered.length && filtered.length>0 ? <CheckSquare size={12}/> : <Square size={12} className="opacity-0"/>}
            </span>
            {selected.size===filtered.length ? "Deselect all" : "Select all"} • {selected.size} selected • {selected.size>0 ? (selectedBytes/1024/1024/1024>=1 ? `${(selectedBytes/1024/1024/1024).toFixed(2)} GiB` : `${(selectedBytes/1024/1024).toFixed(1)} MiB`) : totalDisplay}
          </label>
          <div className="flex gap-2">
            <button onClick={handleDeleteSelected} disabled={selected.size===0 || cleaning} className="inline-flex items-center gap-1.5 rounded-full bg-[#E11D48] px-4 py-2 text-xs font-semibold text-white hover:bg-[#FF3B6A] disabled:opacity-40">
              {cleaning ? <Loader2 size={12} className="animate-spin"/> : <Trash2 size={12}/>} Delete selected
            </button>
            <button onClick={handleDeleteAll} disabled={filtered.length===0 || cleaning} className="inline-flex items-center gap-1.5 rounded-full bg-[#C9A84C] px-5 py-2 text-xs font-bold text-black hover:bg-[#E8C86A] disabled:opacity-40">
              <Zap size={12}/> Delete ALL • {totalDisplay}
            </button>
          </div>
        </div>
      )}

      {loading ? (
        <div className="py-16 flex flex-col items-center gap-3 text-[#6B6661]">
          <Loader2 size={24} className="animate-spin text-[#C9A84C]"/>
          <p className="text-sm">Hunting node_modules, target, venv, dist … across all drives</p>
        </div>
      ) : filtered.length===0 ? (
        <div className="py-16 text-center border border-dashed border-white/10 rounded-2xl bg-[#0A0A0A]">
          <FileStack size={28} className="mx-auto text-[#6B6661]"/>
          <p className="mt-3 text-sm text-white font-medium">No dev artifacts found</p>
          <p className="text-xs text-[#6B6661] mt-1">Clean! No node_modules / target / venv to purge.</p>
        </div>
      ) : (
        <div className="space-y-4 max-h-[52vh] overflow-auto pr-1">
          {grouped.map(([lang, items])=> (
            <div key={lang} className="rounded-xl border border-white/5 bg-[#141414] overflow-hidden">
              <div className="px-4 py-2.5 bg-[#0A0A0A] border-b border-white/5 flex items-center justify-between">
                <span className="text-xs font-semibold tracking-widest uppercase text-[#A8A39E] flex items-center gap-1.5">
                  <span className="w-2 h-2 rounded-full" style={{background: kindMeta[items[0]?.kind]?.color ?? "#6B7280"}}/>
                  {lang} • {items.length} folders
                </span>
                <span className="text-xs text-[#6B6661]">{(items.reduce((a,m)=>a+m.size_bytes,0)/1024/1024/1024).toFixed(2)} GiB</span>
              </div>
              <div className="divide-y divide-white/5">
                {items.map(m=> {
                  const sel = selected.has(m.path);
                  const meta = kindMeta[m.kind] ?? { label: m.kind, color: "#6B7280" };
                  return (
                    <div key={m.path} className={`flex items-center gap-3 px-4 py-3 hover:bg-white/5 ${sel ? "bg-[rgba(201,168,76,0.06)]" : ""}`}>
                      <button onClick={()=>toggle(m.path)} className={`w-5 h-5 rounded-md border flex items-center justify-center shrink-0 ${sel ? "bg-[#C9A84C] border-[#C9A84C] text-black" : "border-white/15 bg-black"}`}>
                        {sel ? <CheckSquare size={12}/> : <Square size={12} className="opacity-0"/>}
                      </button>
                      <div className="w-9 h-9 rounded-lg flex items-center justify-center shrink-0" style={{background: `${meta.color}18`, border: `1px solid ${meta.color}30`, color: meta.color}}>
                        <Folder size={14}/>
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium text-white truncate" title={m.path}>{m.path}</p>
                        <p className="text-xs text-[#6B6661] truncate">{m.name} • {m.kind} • {m.file_count.toLocaleString()} files • <span style={{color: meta.color}}>{m.language}</span></p>
                      </div>
                      <span className="hidden sm:inline text-xs font-medium px-2 py-1 rounded-full border" style={{background: `${meta.color}14`, borderColor: `${meta.color}30`, color: meta.color}}>{m.size_display}</span>
                      <HardDrive size={12} className="hidden sm:block text-[#6B6661]"/>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
