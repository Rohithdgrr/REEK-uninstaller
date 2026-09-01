import { Trash2, RefreshCw, ShieldCheck } from "lucide-react";

export function Header({ onScan, scanning }: { onScan: () => void; scanning: boolean }) {
  return (
    <header className="sticky top-0 z-10 bg-white border-b border-slate-200 px-6 py-4 flex items-center justify-between shadow-sm">
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-xl bg-blue-600 flex items-center justify-center text-white shadow-sm">
          <Trash2 size={18} aria-hidden />
        </div>
        <div>
          <h1 className="text-[15px] font-semibold text-slate-900 leading-none tracking-tight">REEK Uninstaller</h1>
          <p className="text-xs text-slate-500 mt-0.5">Premium system cleanup</p>
        </div>
        <span className="ml-2 hidden sm:inline-flex items-center gap-1.5 rounded-full bg-slate-50 border border-slate-200 px-2.5 py-1 text-xs text-slate-600">
          <ShieldCheck size={12} className="text-green-600" /> Safe uninstall
        </span>
      </div>
      <button
        onClick={onScan}
        disabled={scanning}
        aria-label="Scan applications"
        className="inline-flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors duration-200"
      >
        <RefreshCw size={16} className={scanning ? "animate-spin" : ""} />
        {scanning ? "Scanning..." : "Scan"}
      </button>
    </header>
  );
}
