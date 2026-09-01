import { useEffect, useMemo, useState, useCallback } from "react";
import { Trash2 } from "lucide-react";
import { Header } from "./components/Header";
import { SearchBar } from "./components/SearchBar";
import { AppTable } from "./components/AppTable";
import { ActionBar } from "./components/ActionBar";
import { ConfirmModal } from "./components/ConfirmModal";
import { ProgressView } from "./components/ProgressView";
import { ResultsView } from "./components/ResultsView";
import { Skeleton } from "./components/Skeleton";
import { Toast } from "./components/Toast";
import { useAppStore } from "./store/useAppStore";
import { scanApplications, uninstallApplications, onUninstallProgress } from "./lib/tauri";

export default function App() {
  const {
    view, apps, loading, search, sortKey, sortDir, selected, force,
    showConfirm, progress, logs, results, error,
    setView, setApps, setLoading, setSearch, setShowConfirm, setForce,
    pushLog, setProgress, setResults, setError, resetLogs, clearSelection,
  } = useAppStore();
  const [toast, setToast] = useState<string>("");

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

  const filtered = useMemo(() => {
    let out = [...apps];
    if (search.trim()) {
      const q = search.toLowerCase();
      out = out.filter((a) => a.name.toLowerCase().includes(q) || (a.publisher ?? "").toLowerCase().includes(q));
    }
    out.sort((a, b) => {
      let cmp = 0;
      if (sortKey === "name") cmp = a.name.localeCompare(b.name);
      else if (sortKey === "date") cmp = (a.install_date ?? "").localeCompare(b.install_date ?? "");
      else if (sortKey === "size") cmp = (a.size_bytes ?? 0) - (b.size_bytes ?? 0);
      return sortDir === "asc" ? cmp : -cmp;
    });
    return out;
  }, [apps, search, sortKey, sortDir]);

  const selectedApps = useMemo(() => apps.filter((a) => selected.has(a.id)), [apps, selected]);

  const handleUninstall = async () => {
    setShowConfirm(false);
    resetLogs();
    setView("progress");
    try {
      const res = await uninstallApplications({ ids: Array.from(selected), force });
      setResults(res);
      setView("results");
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

  // Splash
  if (view === "splash" || (loading && apps.length === 0)) {
    return (
      <div className="min-h-screen bg-slate-50 flex flex-col items-center justify-center p-8">
        <div className="w-16 h-16 rounded-2xl bg-blue-600 flex items-center justify-center text-white shadow-md">
          <Trash2 size={28} />
        </div>
        <h1 className="mt-4 text-xl font-semibold text-slate-900">REEK Uninstaller</h1>
        <p className="text-sm text-slate-500 mt-1">Scanning your system…</p>
        <div className="mt-8 w-full max-w-xl">
          <Skeleton />
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-slate-50 flex flex-col">
      <Header onScan={load} scanning={loading} />

      <main className="flex-1 mx-auto w-full max-w-[1200px] px-6 py-6 pb-24 space-y-5">
        {error && (
          <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800">
            {error}
          </div>
        )}

        {view === "dashboard" && (
          <>
            <SearchBar value={search} onChange={setSearch} count={filtered.length} />
            {loading ? <Skeleton /> : <AppTable apps={filtered} />}
          </>
        )}

        {view === "progress" && (
          <ProgressView current={progress?.current ?? 0} total={progress?.total ?? selected.size} logs={logs} />
        )}

        {view === "results" && (
          <ResultsView results={results} onDone={handleDone} />
        )}
      </main>

      {view === "dashboard" && (
        <ActionBar count={selected.size} onUninstall={() => setShowConfirm(true)} onClear={clearSelection} />
      )}

      {showConfirm && (
        <ConfirmModal
          apps={selectedApps}
          force={force}
          onForceChange={setForce}
          onCancel={() => setShowConfirm(false)}
          onConfirm={handleUninstall}
        />
      )}

      <Toast message={toast} onClose={() => setToast("")} />

      {/* Accessibility: live region for progress */}
      <div aria-live="polite" className="sr-only">
        {progress ? `Progress ${progress.current} of ${progress.total}: ${progress.app_name} ${progress.status}` : ""}
      </div>
    </div>
  );
}
