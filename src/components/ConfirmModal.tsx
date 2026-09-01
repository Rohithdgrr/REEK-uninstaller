import { AlertTriangle, ChevronDown } from "lucide-react";
import { useState } from "react";
import type { AppEntry } from "../lib/tauri";

export function ConfirmModal({
  apps,
  force,
  onForceChange,
  onCancel,
  onConfirm,
}: {
  apps: AppEntry[];
  force: boolean;
  onForceChange: (v: boolean) => void;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-slate-900/30 backdrop-blur-sm" onClick={onCancel} aria-hidden />
      <div role="dialog" aria-modal="true" aria-label="Confirm uninstall" className="relative w-full max-w-lg bg-white rounded-2xl shadow-xl border border-slate-200 overflow-hidden">
        <div className="px-6 pt-6">
          <div className="w-10 h-10 rounded-full bg-red-50 border border-red-100 flex items-center justify-center text-red-600">
            <AlertTriangle size={20} />
          </div>
          <h2 className="mt-4 text-lg font-semibold text-slate-900">Are you sure you want to uninstall these applications?</h2>
          <p className="mt-1 text-sm text-slate-600">This action will run each app&apos;s uninstaller. You can force removal if the uninstaller fails.</p>

          <button
            onClick={() => setOpen(!open)}
            className="mt-4 w-full flex items-center justify-between rounded-xl border border-slate-200 bg-slate-50 px-4 py-2.5 text-sm font-medium text-slate-700 hover:bg-white transition"
          >
            <span>{apps.length} applications selected</span>
            <ChevronDown size={16} className={`transition ${open ? "rotate-180" : ""}`} />
          </button>
          {open && (
            <ul className="mt-2 max-h-40 overflow-auto rounded-xl border border-slate-200 divide-y divide-slate-100 bg-white">
              {apps.map((a) => (
                <li key={a.id} className="px-4 py-2 text-sm text-slate-700 flex justify-between">
                  <span className="truncate">{a.name}</span>
                  <span className="text-slate-400 ml-2">{a.version ?? ""}</span>
                </li>
              ))}
            </ul>
          )}

          <label className="mt-4 flex items-center gap-2 text-sm text-slate-700">
            <input
              type="checkbox"
              checked={force}
              onChange={(e) => onForceChange(e.target.checked)}
              className="h-4 w-4 rounded border-slate-300 text-blue-600 focus:ring-blue-600"
            />
            Force removal if uninstaller fails
          </label>
        </div>
        <div className="mt-6 flex justify-end gap-3 bg-slate-50 border-t border-slate-200 px-6 py-4">
          <button onClick={onCancel} className="rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 transition">
            Cancel
          </button>
          <button onClick={onConfirm} className="rounded-xl bg-red-600 px-5 py-2 text-sm font-semibold text-white hover:bg-red-700 shadow-sm transition">
            Confirm Uninstall
          </button>
        </div>
      </div>
    </div>
  );
}
