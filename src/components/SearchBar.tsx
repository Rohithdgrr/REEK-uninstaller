import { Search } from "lucide-react";

export function SearchBar({
  value,
  onChange,
  count,
}: {
  value: string;
  onChange: (v: string) => void;
  count: number;
}) {
  return (
    <div className="flex items-center gap-3">
      <div className="relative flex-1 max-w-xl">
        <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" aria-hidden />
        <input
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="Search applications…"
          aria-label="Search applications"
          className="w-full rounded-xl border border-slate-200 bg-white pl-9 pr-4 py-2.5 text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-600/20 focus:border-blue-600 transition"
        />
      </div>
      <span className="inline-flex items-center rounded-full bg-white border border-slate-200 px-3 py-1.5 text-sm font-medium text-slate-700 shadow-sm whitespace-nowrap">
        Found {count} applications
      </span>
    </div>
  );
}
