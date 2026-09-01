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
    <div className="bg-transparent rounded-[12px] border border-[rgba(225,29,72,0.08)] p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="font-display font-semibold text-[24px] text-[#F5F0EB] flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-[#E11D48] shadow-[0_0_12px_rgba(225,29,72,0.6)] animate-pulse" />
          Uninstalling...
          <span className="text-[16px] font-normal text-[#C9A84C]">({current} of {total})</span>
        </h2>
        <span className="text-[12px] font-medium text-[#6B6661]">{pct}%</span>
      </div>

      {/* Progress Bar 4px glow */}
      <div className="h-1 rounded-full bg-[rgba(255,255,255,0.06)] overflow-hidden">
        <div
          className="h-full bg-[#E11D48] rounded-full transition-all duration-[400ms] ease-out"
          style={{ width: `${pct}%`, boxShadow: "0 0 20px rgba(225,29,72,0.4)" }}
        />
      </div>

      {/* Log Console #080808 JetBrains Mono */}
      <div
        ref={ref}
        className="h-[280px] overflow-auto rounded-[12px] bg-[#080808] border border-[rgba(255,255,255,0.04)] p-4 font-mono text-[13px] leading-[1.8]"
      >
        {logs.length === 0 ? (
          <span className="text-[#6B6661]">Waiting for logs…</span>
        ) : (
          logs.map((l, i) => {
            const isSuccess = l.includes("✓") || l.toLowerCase().includes("success") || l.includes("done");
            const isError = l.includes("✗") || l.toLowerCase().includes("failed") || l.toLowerCase().includes("error");
            const isProcessing = l.includes("🔄") || l.toLowerCase().includes("uninstalling");
            const color = isSuccess ? "text-[#34D399]" : isError ? "text-[#E11D48]" : isProcessing ? "text-[#C9A84C]" : "text-[#A8A39E]";
            // extract timestamp-like [2.4s] muted
            const parts = l.split(/(\[\d+\.?\d*s\])/);
            return (
              <div key={i} className={`log-enter flex gap-2 ${color}`}>
                <span className="text-[#6B6661] shrink-0">&gt;</span>
                <span className="flex-1 break-all">
                  {parts.map((p, idx) =>
                    /^\[\d/.test(p) ? (
                      <span key={idx} className="text-[#6B6661] ml-2">
                        {p}
                      </span>
                    ) : (
                      <span key={idx}>{p}</span>
                    )
                  )}
                </span>
              </div>
            );
          })
        )}
      </div>

      <div className="flex justify-end">
        <button className="rounded-full border border-[rgba(255,255,255,0.08)] px-5 py-1.5 text-[13px] text-[#6B6661] hover:text-[#F5F0EB] transition">Cancel</button>
      </div>
    </div>
  );
}
