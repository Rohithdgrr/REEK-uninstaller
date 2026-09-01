import { CheckCircle2, XCircle, Zap } from "lucide-react";
import { useState } from "react";
import type { UninstallResultDto } from "../lib/tauri";

export function ResultsView({ results, onDone }: { results: UninstallResultDto[]; onDone: () => void }) {
  const success = results.filter((r) => r.success).length;
  const failed = results.length - success;
  const totalTime = "—";
  const [open, setOpen] = useState(failed > 0);

  return (
    <div className="space-y-6">
      <h2 className="font-display font-semibold text-[28px] text-[#C9A84C] flex items-center gap-2">
        <span>✨</span> Uninstallation Complete
      </h2>
      <div className="h-px bg-[rgba(225,29,72,0.08)]" />

      {/* 3-up Grid */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <div className="stagger-1 bg-[#141414] rounded-[12px] border border-[rgba(225,29,72,0.06)] p-5 hover:border-[rgba(225,29,72,0.15)] hover:shadow-[0_0_20px_rgba(225,29,72,0.06)] transition-all">
          <div className="flex items-center gap-2 text-[12px] tracking-[0.3px] uppercase font-semibold text-[#6B6661]">
            <CheckCircle2 size={14} className="text-[#34D399]" /> Removed
          </div>
          <p className="mt-2 text-[36px] font-bold leading-none text-[#34D399]">{success}</p>
        </div>
        <div className="stagger-2 bg-[#141414] rounded-[12px] border border-[rgba(225,29,72,0.06)] p-5 hover:border-[rgba(225,29,72,0.15)] hover:shadow-[0_0_20px_rgba(225,29,72,0.06)] transition-all">
          <div className="flex items-center gap-2 text-[12px] tracking-[0.3px] uppercase font-semibold text-[#6B6661]">
            <XCircle size={14} className="text-[#FF2047]" /> Failed
          </div>
          <p className="mt-2 text-[36px] font-bold leading-none text-[#FF2047]">{failed}</p>
        </div>
        <div className="stagger-3 bg-[#141414] rounded-[12px] border border-[rgba(225,29,72,0.06)] p-5 hover:border-[rgba(225,29,72,0.15)] hover:shadow-[0_0_20px_rgba(225,29,72,0.06)] transition-all">
          <div className="flex items-center gap-2 text-[12px] tracking-[0.3px] uppercase font-semibold text-[#6B6661]">
            <Zap size={14} className="text-[#C9A84C]" /> Time
          </div>
          <p className="mt-2 text-[36px] font-bold leading-none text-[#C9A84C]">{totalTime}</p>
        </div>
      </div>

      {failed > 0 && (
        <div className="bg-[#141414] rounded-[12px] border border-[rgba(225,29,72,0.06)] overflow-hidden">
          <button
            onClick={() => setOpen(!open)}
            className="w-full flex items-center justify-between px-5 py-3 text-[14px] font-medium text-[#F5F0EB] hover:bg-[#1A1A1A] transition"
          >
            <span>Failed applications</span>
            <span className={`transition-transform ${open ? "rotate-180" : ""}`}>⌄</span>
          </button>
          {open && (
            <ul className="divide-y divide-[rgba(255,255,255,0.04)] border-t border-[rgba(225,29,72,0.06)] max-h-64 overflow-auto px-5 py-3 space-y-2 bg-[#141414]">
              {results
                .filter((r) => !r.success)
                .map((r) => (
                  <li key={r.id} className="flex gap-2 text-[13px]">
                    <span className="text-[#E11D48]">•</span>
                    <span className="text-[#FF2047] font-medium">{r.name}</span>
                    <span className="text-[#A8A39E]">— {r.error ?? "unknown error"}</span>
                  </li>
                ))}
            </ul>
          )}
        </div>
      )}

      <div className="flex justify-end">
        <button
          onClick={onDone}
          className="inline-flex items-center gap-1.5 rounded-full bg-[#E11D48] px-8 py-2.5 text-[14px] font-semibold text-white shadow-[0_0_40px_rgba(225,29,72,0.3)] hover:bg-[#FF3B6A] hover:scale-[1.03] transition-all"
        >
          ✨ Done
        </button>
      </div>
    </div>
  );
}
