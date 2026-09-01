import { Trash2 } from "lucide-react";

export function ActionBar({
  count,
  onUninstall,
  onClear,
}: {
  count: number;
  onUninstall: () => void;
  onClear: () => void;
}) {
  return (
    <div className="fixed bottom-0 left-0 right-0 bg-white border-t border-slate-200 shadow-[0_-8px_24px_rgba(15,23,42,0.06)]">
      <div className="mx-auto max-w-[1200px] px-6 py-4 flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-slate-900">{count} applications selected</span>
          {count > 0 && (
            <button onClick={onClear} className="text-sm text-slate-600 hover:text-slate-900 underline-offset-2 hover:underline">
              Deselect all
            </button>
          )}
        </div>
        <button
          onClick={onUninstall}
          disabled={count === 0}
          aria-label="Uninstall selected"
          className="inline-flex items-center gap-2 rounded-xl bg-blue-600 px-6 py-2.5 text-sm font-semibold text-white shadow-sm hover:bg-blue-700 disabled:bg-slate-200 disabled:text-slate-500 disabled:cursor-not-allowed transition-colors duration-200"
        >
          <Trash2 size={16} /> Uninstall Selected
        </button>
      </div>
    </div>
  );
}
