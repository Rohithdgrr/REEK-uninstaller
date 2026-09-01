import { useEffect, useRef } from "react";

export function ProgressView({
  current,
  total,
  logs,
}: {
  current: number;
  total: number;
  logs: string[];
}) {
  const pct = total === 0 ? 0 : Math.round((current / total) * 100);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [logs]);

  return (
    <div className="bg-white rounded-2xl border border-slate-200 shadow-sm p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold text-slate-900">Uninstalling…</h2>
        <span className="inline-flex items-center rounded-full bg-blue-50 border border-blue-100 px-3 py-1 text-xs font-semibold text-blue-700">
          Processing {current} of {total}
        </span>
      </div>
      <div className="h-2 rounded-full bg-slate-100 overflow-hidden">
        <div className="h-full bg-blue-600 transition-all duration-300" style={{ width: `${pct}%` }} />
      </div>
      <div className="flex justify-between text-xs text-slate-500">
        <span>{pct}% complete</span>
        <span>{current}/{total}</span>
      </div>
      <div ref={ref} className="h-64 overflow-auto rounded-xl bg-slate-900 text-slate-100 font-mono text-xs p-4 leading-relaxed border border-slate-800">
        {logs.length === 0 ? <span className="text-slate-400">Waiting for logs…</span> : logs.map((l, i) => <div key={i}>{l}</div>)}
      </div>
    </div>
  );
}
