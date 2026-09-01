import { useEffect, useState, useMemo } from "react";
import { Film, HardDrive, Trash2, Play, Loader2, Search, FolderOpen, Clock, CheckSquare, Square } from "lucide-react";
import { scanVideos, deleteVideos, type VideoEntryDto } from "../lib/tauri";

export function VideoVault({ onDeleted }: { onDeleted?: (count: number) => void }) {
  const [videos, setVideos] = useState<VideoEntryDto[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [deleting, setDeleting] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const v = await scanVideos();
      setVideos(v);
    } catch (e) {
      console.error(e);
      setVideos([]);
    } finally { setLoading(false); }
  };

  useEffect(() => { load(); }, []);

  const filtered = useMemo(() => {
    if (!videos) return [];
    let out = videos;
    if (query.trim()) {
      const q = query.toLowerCase();
      out = out.filter(v => v.name.toLowerCase().includes(q) || v.path.toLowerCase().includes(q));
    }
    return out;
  }, [videos, query]);

  const grouped = useMemo(() => {
    const map = new Map<string, VideoEntryDto[]>();
    for (const v of filtered) {
      const key = v.drive ? `${v.drive}:` : "Other";
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(v);
    }
    return Array.from(map.entries()).sort((a,b)=> a[0].localeCompare(b[0]));
  }, [filtered]);

  const totalSize = useMemo(() => filtered.reduce((a, v) => a + v.size_bytes, 0), [filtered]);
  const totalDisplay = totalSize > 0 ? (totalSize / 1024/1024/1024 >= 1 ? `${(totalSize/1024/1024/1024).toFixed(2)} GiB` : `${(totalSize/1024/1024).toFixed(1)} MiB`) : "—";

  const toggle = (path: string) => {
    setSelected(s => {
      const n = new Set(s);
      if (n.has(path)) n.delete(path); else n.add(path);
      return n;
    });
  };
  const toggleAll = () => {
    if (selected.size === filtered.length) setSelected(new Set());
    else setSelected(new Set(filtered.map(v => v.path)));
  };

  const handleDelete = async () => {
    if (selected.size === 0) return;
    setDeleting(true);
    try {
      const paths = Array.from(selected);
      await deleteVideos(paths);
      const remaining = (videos ?? []).filter(v => !selected.has(v.path));
      setVideos(remaining);
      setSelected(new Set());
      onDeleted?.(paths.length);
    } catch (e) { console.error(e); }
    finally { setDeleting(false); }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-xl bg-[rgba(225,29,72,0.12)] border border-[rgba(225,29,72,0.18)] flex items-center justify-center text-[#E11D48]"><Film size={18} /></div>
          <div>
            <h3 className="text-[16px] font-semibold text-white tracking-tight">Movies • Videos</h3>
            <p className="text-[11px] text-[#6B6661]">{loading ? "Scanning whole device…" : `${filtered.length} videos • ${totalDisplay} total`}</p>
          </div>
        </div>
        <button onClick={load} disabled={loading} className="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-[#141414] px-3 py-1.5 text-xs text-[#A8A39E] hover:text-white disabled:opacity-50">
          {loading ? <Loader2 size={12} className="animate-spin"/> : <Search size={12}/>} {loading ? "Scanning…" : "Rescan"}
        </button>
      </div>

      <div className="flex gap-2">
        <div className="flex-1 relative">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#6B6661]" />
          <input value={query} onChange={e=>setQuery(e.target.value)} placeholder="Search videos by name or path…" className="w-full bg-[#0A0A0A] border border-white/10 rounded-full pl-9 pr-4 py-2.5 text-sm text-white placeholder:text-[#6B6661] focus:outline-none focus:border-[rgba(225,29,72,0.3)]" />
        </div>
        {filtered.length>0 && (
          <button onClick={toggleAll} className="px-4 py-2 rounded-full border border-white/10 bg-[#141414] text-xs text-white hover:bg-white hover:text-black">
            {selected.size===filtered.length ? "Deselect all" : "Select all"}
          </button>
        )}
      </div>

      {selected.size>0 && (
        <div className="flex items-center justify-between bg-[rgba(225,29,72,0.08)] border border-[rgba(225,29,72,0.18)] rounded-xl px-4 py-3">
          <span className="text-sm text-white font-medium">{selected.size} selected • {Array.from(selected).length} videos will be moved to recycle bin</span>
          <button onClick={handleDelete} disabled={deleting} className="inline-flex items-center gap-1.5 rounded-full bg-[#E11D48] px-5 py-2 text-sm font-semibold text-white hover:bg-[#FF3B6A] disabled:opacity-50">
            {deleting ? <Loader2 size={14} className="animate-spin"/> : <Trash2 size={14}/>} Delete selected
          </button>
        </div>
      )}

      {loading ? (
        <div className="py-16 flex flex-col items-center gap-3 text-[#6B6661]">
          <Loader2 size={24} className="animate-spin text-[#E11D48]"/>
          <p className="text-sm">Scanning every drive for .mp4 .mkv .avi …</p>
          <p className="text-xs">This scans Videos, Downloads, Desktop and all drives (depth 4) — accurate sizes.</p>
        </div>
      ) : filtered.length===0 ? (
        <div className="py-16 text-center border border-dashed border-white/10 rounded-2xl bg-[#0A0A0A]">
          <Film size={28} className="mx-auto text-[#6B6661]"/>
          <p className="mt-3 text-sm text-white font-medium">No videos found</p>
          <p className="text-xs text-[#6B6661] mt-1">Try rescan — we look in Videos, Downloads and every drive.</p>
        </div>
      ) : (
        <div className="space-y-6 max-h-[52vh] overflow-auto pr-1">
          {grouped.map(([drive, items])=> (
            <div key={drive} className="rounded-xl border border-white/5 bg-[#141414] overflow-hidden">
              <div className="px-4 py-2.5 bg-[#0A0A0A] border-b border-white/5 flex items-center justify-between">
                <span className="text-xs font-semibold tracking-widest uppercase text-[#A8A39E] flex items-center gap-1.5"><HardDrive size={12}/> {drive} • {items.length} videos</span>
                <span className="text-xs text-[#6B6661]">{(items.reduce((a,v)=>a+v.size_bytes,0)/1024/1024/1024).toFixed(2)} GiB</span>
              </div>
              <div className="divide-y divide-white/5">
                {items.map(v=> {
                  const sel = selected.has(v.path);
                  return (
                    <div key={v.path} className={`flex items-center gap-3 px-4 py-3 hover:bg-white/5 ${sel ? "bg-[rgba(225,29,72,0.06)]" : ""}`}>
                      <button onClick={()=>toggle(v.path)} className={`w-5 h-5 rounded-md border flex items-center justify-center shrink-0 ${sel ? "bg-[#E11D48] border-[#E11D48] text-white" : "border-white/15 bg-black text-transparent"}`}>
                        {sel ? <CheckSquare size={12}/> : <Square size={12} className="opacity-0"/>}
                      </button>
                      <div className="w-12 h-8 rounded bg-black border border-white/10 flex items-center justify-center shrink-0 overflow-hidden">
                        <Film size={14} className="text-[#6B6661]"/>
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium text-white truncate" title={v.path}>{v.name}</p>
                        <p className="text-xs text-[#6B6661] truncate flex items-center gap-1.5"><FolderOpen size={11}/> {v.path} <span className="hidden sm:inline">• {v.extension.toUpperCase()}</span></p>
                      </div>
                      <span className="hidden sm:inline text-xs font-medium text-[#C9A84C] border border-[rgba(201,168,76,0.18)] bg-[rgba(201,168,76,0.08)] px-2 py-1 rounded-full">{v.size_display}</span>
                      <a href={`file://${v.path}`} target="_blank" rel="noreferrer" className="hidden sm:inline-flex w-8 h-8 rounded-full bg-white text-black items-center justify-center hover:scale-105" title="Open">
                        <Play size={12} fill="black"/>
                      </a>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      )}

      <p className="text-[11px] text-[#6B6661] flex items-center gap-1.5"><Clock size={12}/> Videos are moved to recycle bin, not permanently deleted — you can restore from Recycle Bin.</p>
    </div>
  );
}
