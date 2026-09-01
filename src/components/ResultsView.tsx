import { CheckCircle2, XCircle, ChevronDown } from "lucide-react";
import { useState } from "react";
import type { UninstallResultDto } from "../lib/tauri";

export function ResultsView({ results, onDone }: { results: UninstallResultDto[]; onDone: () => void }) {
  const success = results.filter((r) => r.success).length;
  const failed = results.length - success;
  const [open, setOpen] = useState(failed > 0);

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div className="bg-white rounded-2xl border border-slate-200 shadow-sm p-6 flex items-center gap-4">
          <div className="w-12 h-12 rounded-full bg-green-50 border border-green-100 flex items-center justify-center text-green-600">
            <CheckCircle2 size={24} />
          </div>
          <div>
            <p className="text-2xl font-bold text-slate-900">{success}</p>
            <p className="text-sm text-slate-600">Successfully removed</p>
          </div>
        </div>
        <div className="bg-white rounded-2xl border border-slate-200 shadow-sm p-6 flex items-center gap-4">
          <div className="w-12 h-12 rounded-full bg-red-50 border border-red-100 flex items-center justify-center text-red-600">
            <XCircle size={24} />
          </div>
          <div>
            <p className="text-2xl font-bold text-slate-900">{failed}</p>
            <p className="text-sm text-slate-600">Failed</p>
          </div>
        </div>
      </div>

      {failed > 0 && (
        <div className="bg-white rounded-2xl border border-slate-200 shadow-sm overflow-hidden">
          <button onClick={() => setOpen(!open)} className="w-full flex items-center justify-between px-6 py-4 text-sm font-medium text-slate-700 hover:bg-slate-50 transition">
            <span>Failed uninstalls ({failed})</span>
            <ChevronDown size={16} className={`${open ? "rotate-180" : ""} transition`} />
          </button>
          {open && (
            <ul className="divide-y divide-slate-100 border-t border-slate-200 max-h-64 overflow-auto">
              {results.filter((r) => !r.success).map((r) => (
                <li key={r.id} className="px-6 py-3">
                  <p className="text-sm font-medium text-slate-900">{r.name}</p>
                  <p className="text-xs text-red-600 mt-1">{r.error}</p>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      <button onClick={onDone} className="w-full rounded-xl bg-blue-600 py-3 text-sm font-semibold text-white shadow-sm hover:bg-blue-700 transition">
        Done — return to dashboard
      </button>
    </div>
  );
}
