export function Skeleton() {
  return (
    <div className="bg-white rounded-2xl border border-slate-200 shadow-sm p-6 space-y-3 animate-pulse">
      <div className="h-6 bg-slate-100 rounded w-1/3" />
      <div className="space-y-2">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="h-12 bg-slate-50 rounded-xl border border-slate-100" />
        ))}
      </div>
    </div>
  );
}
