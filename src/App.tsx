import { useEffect, useMemo, useState, useCallback } from "react";
import { Header } from "./components/Header";
import { SearchBar } from "./components/SearchBar";
import { AppTable } from "./components/AppTable";
import { ActionBar } from "./components/ActionBar";
import { ConfirmModal } from "./components/ConfirmModal";
import { ProgressView } from "./components/ProgressView";
import { ResultsView } from "./components/ResultsView";
import { Skeleton } from "./components/Skeleton";
import { Toast } from "./components/Toast";
import { SystemStatsBar } from "./components/SystemStatsBar";
import { AppDetailsDrawer } from "./components/AppDetailsDrawer";
import { VideoVault } from "./components/VideoVault";
import { DevCleaner } from "./components/DevCleaner";
import { SuccessTickDialog } from "./components/SuccessTickDialog";
import { useAppStore } from "./store/useAppStore";
import { scanApplications, uninstallApplications, onUninstallProgress, getAppResources, type AppResourceDto } from "./lib/tauri";
import { Film, Package, LayoutGrid } from "lucide-react";

export default function App() {
  const {
    view, apps, loading, search, sortKey, sortDir, selected, force,
    showConfirm, progress, logs, results, error,
    setView, setApps, setLoading, setSearch, setShowConfirm, setForce,
    pushLog, setProgress, setResults, setError, resetLogs, clearSelection,
  } = useAppStore();
  const [toast, setToast] = useState<string>("");
  const [detailId, setDetailId] = useState<string | null>(null);
  const [resMap, setResMap] = useState<Record<string, AppResourceDto>>({});
  const [section, setSection] = useState<"apps" | "movies" | "dev">("apps");
  const [success, setSuccess] = useState<{ title: string; subtitle?: string; details?: string } | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await scanApplications();
      setApps(data);
      setView("dashboard");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setToast(`Scan failed: ${msg}`);
    } finally {
      setLoading(false);
    }
  }, [setApps, setLoading, setError, setView]);

  useEffect(() => {
    load();
    let unlisten: (() => void) | undefined;
    onUninstallProgress((evt) => {
      setProgress(evt);
      pushLog(`[${evt.current}/${evt.total}] ${evt.app_name} — ${evt.status}: ${evt.log}`);
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [load, setProgress, pushLog]);

  // Live resources for default heavy-first sorting (size + CPU/GPU/RAM/VRAM)
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const m = await getAppResources();
        if (alive) setResMap(m);
      } catch {}
    };
    poll();
    const id = setInterval(poll, 3000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  const filtered = useMemo(() => {
    let out = [...apps];
    if (search.trim()) {
      const q = search.toLowerCase();
      out = out.filter((a) => a.name.toLowerCase().includes(q) || (a.publisher ?? "").toLowerCase().includes(q));
    }
    const resourceScore = (id: string) => {
      const r = resMap[id];
      if (!r) return 0;
      // Weighted: RAM + VRAM in MiB + CPU*12 + GPU*8 + running boost
      const memMiB = (r.memory_bytes ?? 0) / (1024 * 1024);
      const vramMiB = (r.vram_bytes ?? 0) / (1024 * 1024);
      const cpu = r.cpu ?? 0;
      const gpu = r.gpu ?? 0;
      const runningBoost = r.is_running ? 80 : 0;
      return memMiB * 1.2 + vramMiB * 0.8 + cpu * 12 + gpu * 8 + runningBoost;
    };
    out.sort((a, b) => {
      let cmp = 0;
      if (sortKey === "name") cmp = a.name.localeCompare(b.name);
      else if (sortKey === "date") cmp = (a.install_date ?? "").localeCompare(b.install_date ?? "");
      else if (sortKey === "size") {
        // Primary by storage size, tie-break by live resource consumption
        cmp = (a.size_bytes ?? 0) - (b.size_bytes ?? 0);
        if (cmp === 0) cmp = resourceScore(a.id) - resourceScore(b.id);
      } else if (sortKey === "resources") {
        // Heavy-first: CPU/GPU/RAM/VRAM + running + size as tie-break
        cmp = resourceScore(a.id) - resourceScore(b.id);
        if (cmp === 0) cmp = (a.size_bytes ?? 0) - (b.size_bytes ?? 0);
      }
      return sortDir === "asc" ? cmp : -cmp;
    });
    return out;
  }, [apps, search, sortKey, sortDir, resMap]);

  const selectedApps = useMemo(() => apps.filter((a) => selected.has(a.id)), [apps, selected]);

  const handleUninstall = async () => {
    setShowConfirm(false);
    resetLogs();
    setView("progress");
    try {
      const res = await uninstallApplications({ ids: Array.from(selected), force });
      setResults(res);
      setView("results");
      const ok = res.filter(r=>r.success).length;
      if (ok>0) setSuccess({ title: "Uninstalled Successfully", subtitle: `${ok} application${ok>1?"s":""} removed`, details: `${ok} removed • ${res.length - ok} failed` });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setToast(msg);
      setView("dashboard");
    }
  };

  const handleDone = async () => {
    clearSelection();
    setView("dashboard");
    await load();
  };

  if (view === "splash" || (loading && apps.length === 0)) {
    return (
      <div className="min-h-screen bg-[#0A0A0A] flex flex-col items-center justify-center p-6 md:p-8 relative overflow-hidden">
        {/* Whole-screen red shade — rich Mahakali aura */}
        <div className="absolute inset-0 pointer-events-none bg-[radial-gradient(1100px_700px_at_50%_50%,rgba(225,29,72,0.16),rgba(225,29,72,0.07)_38%,transparent_72%)]" style={{ animation: "pulse-ring 4s ease-in-out infinite" }} />
        <div className="absolute inset-0 pointer-events-none bg-[linear-gradient(180deg,rgba(225,29,72,0.06)_0%,transparent_28%,transparent_72%,rgba(225,29,72,0.08)_100%)]" />
        <div className="absolute inset-0 pointer-events-none opacity-40" style={{ background: "radial-gradient(900px 520px at 70% 60%, rgba(225,29,72,0.09), transparent 70%)" }} />
        <div className="absolute inset-0 void-glow pointer-events-none opacity-60" />

        {/* Title */}
        <div className="relative flex flex-col items-center text-center">
          <h1 className="font-display font-bold text-[32px] md:text-[34px] tracking-[0.08em] text-[#F5F0EB]">MAHAKALI</h1>
          <p className="font-sans font-medium text-[11px] tracking-[0.24em] uppercase text-[#E11D48] mt-1">THE OMNI DESTRUCTOR GOD</p>
        </div>

        {/* Animated Mahakali — replaces skeleton — NO ROTATION, rich static ritual */}
        <div className="relative mt-8 flex flex-col items-center">
          {/* ambient glow — soft breathing, no rotation */}
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[520px] h-[520px] md:w-[620px] md:h-[620px] rounded-full bg-[radial-gradient(circle_at_center,rgba(225,29,72,0.14),transparent_65%)] blur-[1px] pointer-events-none" style={{ animation: "pulse-ring 3s ease-in-out infinite" }} />

          {/* halo rings — static breathing, no spin */}
          <div className="absolute top-1/2 left-1/2 w-[440px] h-[440px] md:w-[500px] md:h-[500px] -translate-x-1/2 -translate-y-1/2 pointer-events-none hidden md:block">
            <div className="absolute inset-0 rounded-full border border-[rgba(225,29,72,0.07)]" style={{ animation: "pulse-ring 3.6s ease-in-out infinite" }} />
            <div className="absolute inset-[18px] rounded-full border border-dashed border-[rgba(225,29,72,0.08)]" style={{ animation: "pulse-ring 4.2s ease-in-out infinite reverse" }} />
            {/* static accent dots with pulse, not orbit */}
            <span className="absolute top-[10%] left-1/2 -translate-x-1/2 w-2 h-2 bg-[#E11D48] rounded-full shadow-[0_0_10px_rgba(225,29,72,0.9)]" style={{ animation: "eye-pulse 2.2s ease-in-out infinite" }} />
            <span className="absolute bottom-[10%] left-1/2 -translate-x-1/2 w-1.5 h-1.5 bg-[#C9A84C] rounded-full shadow-[0_0_8px_rgba(201,168,76,0.7)]" style={{ animation: "eye-pulse 2.8s ease-in-out infinite 0.6s" }} />
            <span className="absolute top-1/2 left-[6%] -translate-y-1/2 w-1.5 h-1.5 bg-[#E11D48]/70 rounded-full" style={{ animation: "bracket-pulse 2.4s ease-in-out infinite" }} />
            <span className="absolute top-1/2 right-[6%] -translate-y-1/2 w-1.5 h-1.5 bg-[#C9A84C]/70 rounded-full" style={{ animation: "bracket-pulse 2.4s ease-in-out infinite 1.2s" }} />
          </div>

          {/* static ritual border — breathing glow, no rotation */}
          <div
            className="relative w-[320px] h-[400px] md:w-[380px] md:h-[460px] rounded-[28px] p-[1.5px] overflow-hidden shadow-[0_0_40px_rgba(225,29,72,0.18),0_0_80px_rgba(225,29,72,0.08)]"
            style={{ background: "linear-gradient(180deg, rgba(225,29,72,0.5) 0%, rgba(225,29,72,0.08) 35%, rgba(225,29,72,0.08) 65%, rgba(225,29,72,0.5) 100%)", animation: "breathe 3s ease-in-out infinite" }}
          >
            <div className="w-full h-full rounded-[26px] overflow-hidden bg-black relative">
              {/* image — clean, no eye augmentation */}
              <img src="/mahakali.png" alt="Mahakali" className="w-full h-full object-cover object-center scale-[1.02]" />

              {/* soft inner haze */}
              <div className="absolute inset-0 bg-[radial-gradient(520px_340px_at_50%_30%,rgba(225,29,72,0.10),transparent_68%)] pointer-events-none" style={{ animation: "pulse-ring 4s ease-in-out infinite" }} />

              {/* corner brackets */}
              <span className="absolute top-3 left-3 w-8 h-8 border-l-[2px] border-t-[2px] border-[#E11D48]/60 rounded-tl-[14px] pointer-events-none" style={{ animation: "bracket-pulse 2.8s ease-in-out infinite" }} />
              <span className="absolute top-3 right-3 w-8 h-8 border-r-[2px] border-t-[2px] border-[#E11D48]/60 rounded-tr-[14px] pointer-events-none" style={{ animation: "bracket-pulse 2.8s ease-in-out infinite 0.4s" }} />
              <span className="absolute bottom-3 left-3 w-8 h-8 border-l-[2px] border-b-[2px] border-[#E11D48]/60 rounded-bl-[14px] pointer-events-none" style={{ animation: "bracket-pulse 2.8s ease-in-out infinite 0.8s" }} />
              <span className="absolute bottom-3 right-3 w-8 h-8 border-r-[2px] border-b-[2px] border-[#E11D48]/60 rounded-br-[14px] pointer-events-none" style={{ animation: "bracket-pulse 2.8s ease-in-out infinite 1.2s" }} />

              {/* bottom shimmer progress */}
              <div className="absolute bottom-0 inset-x-0 h-[3px] bg-[rgba(255,255,255,0.06)] overflow-hidden">
                <div className="absolute inset-y-0 w-[45%] bg-gradient-to-r from-transparent via-[#E11D48] to-transparent" style={{ animation: "shimmer 1.6s ease-in-out infinite" }} />
              </div>

              {/* vignette */}
              <div className="absolute inset-0 rounded-[26px] shadow-[inset_0_0_60px_rgba(0,0,0,0.7)] pointer-events-none" />
            </div>
          </div>

          {/* status */}
          <div className="mt-7 flex flex-col items-center gap-2 text-center">
            <p className="inline-flex items-center gap-2 rounded-full bg-[#141414] border border-[rgba(225,29,72,0.12)] px-4 py-1.5 text-[11px] font-medium tracking-[0.14em] uppercase text-[#A8A39E]">
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[#E11D48] opacity-75" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-[#E11D48] shadow-[0_0_8px_rgba(225,29,72,0.8)]" />
              </span>
              Scanning your system…
            </p>
            <p className="text-[13px] text-[#6B6661]">
              Mahakali is purifying — detecting artifacts, registry phantoms &amp; leftover souls
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[#0A0A0A] flex flex-col relative overflow-hidden">
      {/* Blueprint void glow at 70% 60% */}
      <div className="pointer-events-none absolute inset-0 void-glow" />
      <div className="pointer-events-none absolute inset-x-0 top-0 h-[420px] glow-red-top opacity-60" />

      <Header onScan={load} scanning={loading} />

      {/* Canvas 1200×800 centered — ONE unified card */}
      <main className="flex-1 mx-auto w-full max-w-[1200px] px-6 py-6 pb-24 relative">
        <div className="rounded-[16px] border border-[rgba(225,29,72,0.08)] bg-[#141414] overflow-hidden shadow-[0_8px_40px_rgba(0,0,0,0.5),0_0_0_1px_rgba(225,29,72,0.04)]">
          {/* System Vitality + Storage — top section of the same card */}
          <SystemStatsBar />

          {/* Divider */}
          <div className="h-px bg-[rgba(225,29,72,0.06)]" />

          {/* Apps / Progress / Results — bottom section of SAME card */}
          <div className="p-6 space-y-4 bg-[#0A0A0A]/40">
            {error && (
              <div className="rounded-[12px] border border-[rgba(225,29,72,0.25)] bg-[rgba(225,29,72,0.08)] px-4 py-3 text-sm text-[#FF2047]">
                {error}
              </div>
            )}

            {view === "dashboard" && (
              <>
                {/* Section tabs — Apps / Movies / Dev Cleaner */}
                <div className="flex items-center gap-2 p-1 rounded-full bg-[#0A0A0A] border border-[rgba(225,29,72,0.08)] w-fit">
                  <button onClick={()=>setSection("apps")} className={`inline-flex items-center gap-1.5 px-4 py-2 rounded-full text-xs font-semibold transition ${section==="apps" ? "bg-[#E11D48] text-white shadow-[0_0_12px_rgba(225,29,72,0.3)]" : "text-[#A8A39E] hover:text-white"}`}>
                    <LayoutGrid size={14}/> Apps {section==="apps" && `• ${filtered.length}`}
                  </button>
                  <button onClick={()=>setSection("movies")} className={`inline-flex items-center gap-1.5 px-4 py-2 rounded-full text-xs font-semibold transition ${section==="movies" ? "bg-[#E11D48] text-white shadow-[0_0_12px_rgba(225,29,72,0.3)]" : "text-[#A8A39E] hover:text-white"}`}>
                    <Film size={14}/> Movies
                  </button>
                  <button onClick={()=>setSection("dev")} className={`inline-flex items-center gap-1.5 px-4 py-2 rounded-full text-xs font-semibold transition ${section==="dev" ? "bg-[#C9A84C] text-black shadow-[0_0_12px_rgba(201,168,76,0.3)]" : "text-[#A8A39E] hover:text-white"}`}>
                    <Package size={14}/> Dev Cleaner
                  </button>
                </div>

                {section==="apps" && (
                  <>
                    <SearchBar value={search} onChange={setSearch} count={filtered.length} />
                    {loading ? <Skeleton /> : <AppTable apps={filtered} onDetails={setDetailId} />}
                  </>
                )}
                {section==="movies" && (
                  <VideoVault onDeleted={(n)=> setSuccess({ title: "Videos Deleted", subtitle: `${n} video${n>1?"s":""} moved to recycle bin`, details: "UPI-style success — you can restore from Recycle Bin" })} />
                )}
                {section==="dev" && (
                  <DevCleaner onCleaned={(c,b)=> {
                    const mib = b/1024/1024;
                    const disp = mib>=1024 ? `${(mib/1024).toFixed(2)} GiB` : `${mib.toFixed(1)} MiB`;
                    setSuccess({ title: "Dev Artifacts Purged", subtitle: `${c} modules cleaned • ${disp} reclaimed`, details: "node_modules, target, venv, dist — reinstall via npm/pip/cargo" });
                  }} />
                )}
              </>
            )}

            {view === "progress" && (
              <ProgressView current={progress?.current ?? 0} total={progress?.total ?? selected.size} logs={logs} />
            )}

            {view === "results" && (
              <ResultsView results={results} onDone={handleDone} />
            )}
          </div>
        </div>
      </main>

      {view === "dashboard" && section==="apps" && (
        <ActionBar count={selected.size} onUninstall={() => setShowConfirm(true)} onClear={clearSelection} />
      )}

      <SuccessTickDialog open={!!success} title={success?.title ?? ""} subtitle={success?.subtitle} details={success?.details} onClose={()=>setSuccess(null)} />

      {showConfirm && (
        <ConfirmModal
          apps={selectedApps}
          force={force}
          onForceChange={setForce}
          onCancel={() => setShowConfirm(false)}
          onConfirm={handleUninstall}
        />
      )}

      <AppDetailsDrawer id={detailId} onClose={() => setDetailId(null)} onUninstalled={load} />

      <Toast message={toast} onClose={() => setToast("")} />

      <div aria-live="polite" className="sr-only">
        {progress ? `Progress ${progress.current} of ${progress.total}: ${progress.app_name} ${progress.status}` : ""}
      </div>
    </div>
  );
}
