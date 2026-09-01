import { ArrowUpDown, Package } from "lucide-react";
import type { AppEntry } from "../lib/tauri";
import { useAppStore } from "../store/useAppStore";

type Props = {
  apps: AppEntry[];
};

export function AppTable({ apps, onDetails }: Props & { onDetails?: (id: string) => void }) {
  const { selected, toggleSelect, toggleSelectAll, sortKey, sortDir, setSort } = useAppStore();
  const visibleIds = apps.map((a) => a.id);
  const allChecked = visibleIds.length > 0 && visibleIds.every((id) => selected.has(id));
  const indeterminate = !allChecked && visibleIds.some((id) => selected.has(id));

  return (
    <div className="bg-white rounded-2xl border border-slate-200 shadow-sm overflow-hidden">
      <div className="overflow-auto max-h-[52vh]">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-slate-50 border-b border-slate-200 text-slate-600">
            <tr>
              <th className="w-10 px-4 py-3 text-left">
                <input
                  type="checkbox"
                  aria-label="Select all"
                  checked={allChecked}
                  ref={(el) => { if (el) el.indeterminate = indeterminate; }}
                  onChange={() => toggleSelectAll(visibleIds)}
                  className="h-4 w-4 rounded border-slate-300 text-blue-600 focus:ring-blue-600"
                />
              </th>
              <th className="text-left font-medium">
                <button onClick={() => setSort("name")} className="inline-flex items-center gap-1.5 py-3 hover:text-slate-900 transition">
                  App Name <ArrowUpDown size={14} className={sortKey === "name" ? "text-blue-600" : "text-slate-400"} />
                </button>
              </th>
              <th className="text-left font-medium hidden md:table-cell">Version</th>
              <th className="text-left font-medium hidden sm:table-cell">Size</th>
              <th className="text-left font-medium hidden lg:table-cell">
                <button onClick={() => setSort("date")} className="inline-flex items-center gap-1.5 py-3 hover:text-slate-900 transition">
                  Installed <ArrowUpDown size={14} className={sortKey === "date" ? "text-blue-600" : "text-slate-400"} />
                </button>
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100">
            {apps.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-6 py-16 text-center">
                  <div className="mx-auto w-12 h-12 rounded-full bg-slate-50 border border-slate-200 flex items-center justify-center text-slate-400">
                    <Package size={20} />
                  </div>
                  <p className="mt-3 text-sm font-medium text-slate-700">No applications found</p>
                  <p className="text-xs text-slate-500">Try a different search or run Scan again.</p>
                </td>
              </tr>
            ) : (
              apps.map((app) => (
                <tr
                  key={app.id}
                  onClick={() => onDetails?.(app.id)}
                  className={`hover:bg-slate-50 transition cursor-pointer ${selected.has(app.id) ? "bg-blue-50/60" : ""}`}
                  title="Click for details"
                >
                  <td className="px-4 py-3" onClick={e => e.stopPropagation()}>
                    <input
                      type="checkbox"
                      aria-label={`Select ${app.name}`}
                      checked={selected.has(app.id)}
                      onChange={() => toggleSelect(app.id)}
                      className="h-4 w-4 rounded border-slate-300 text-blue-600 focus:ring-blue-600"
                    />
                  </td>
                  <td className="py-3 pr-4">
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-lg bg-slate-100 border border-slate-200 flex items-center justify-center text-slate-500 text-xs font-semibold">
                        {app.name.slice(0, 2).toUpperCase()}
                      </div>
                      <div className="min-w-0">
                        <p className="font-medium text-slate-900 truncate">{app.name}</p>
                        <p className="text-xs text-slate-500 truncate">{app.publisher ?? app.source_label}</p>
                      </div>
                    </div>
                  </td>
                  <td className="py-3 text-slate-700 hidden md:table-cell">{app.version ?? "—"}</td>
                  <td className="py-3 text-slate-700 hidden sm:table-cell">{app.size_display ?? "—"}</td>
                  <td className="py-3 text-slate-600 hidden lg:table-cell">{app.install_date ?? "—"}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
