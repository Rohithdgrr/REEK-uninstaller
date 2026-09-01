import { ScanSearch, Settings2, Sparkles } from "lucide-react";

export function Header({ onScan, scanning }: { onScan: () => void; scanning: boolean }) {
  return (
    <header className="sticky top-0 z-20 bg-[#0A0A0A]/90 backdrop-blur-[12px] border-b border-[rgba(225,29,72,0.08)] h-[64px] flex items-center">
      <div className="mx-auto w-full max-w-[1200px] px-6 flex items-center justify-between gap-4">
        {/* Left — wordmark only (logo removed per request), with subtle ritual accent */}
        <div className="flex items-center gap-3">
          <div className="hidden sm:block w-px h-8 bg-gradient-to-b from-transparent via-[rgba(225,29,72,0.4)] to-transparent" aria-hidden />
          <div className="leading-none">
            <h1 className="font-display font-bold text-[22px] tracking-[0.08em] leading-none text-[#F5F0EB] inline-flex items-center gap-1.5">
              MAHAKALI
              <Sparkles size={12} className="text-[#E11D48] opacity-70 hidden sm:inline" />
            </h1>
            <p className="font-sans font-medium text-[10px] tracking-[0.22em] uppercase text-[#E11D48] mt-[3px]">THE OMNI DESTRUCTOR GOD</p>
          </div>
        </div>

        <div className="flex items-center gap-2.5">
          <button
            onClick={onScan}
            disabled={scanning}
            aria-label="Scan applications"
            className="group inline-flex items-center gap-2.5 rounded-full bg-[#1A1A1A] border border-[rgba(225,29,72,0.16)] pl-1.5 pr-4 py-1.5 text-[13px] font-semibold tracking-[0.02em] text-[#F5F0EB] hover:bg-[#1E1E1E] hover:border-[rgba(225,29,72,0.28)] hover:shadow-[0_0_20px_rgba(225,29,72,0.18)] disabled:opacity-40 disabled:cursor-not-allowed transition-all"
          >
            <span className="w-7 h-7 rounded-full bg-[rgba(225,29,72,0.14)] border border-[rgba(225,29,72,0.22)] flex items-center justify-center text-[#E11D48] group-hover:bg-[rgba(225,29,72,0.2)] transition-colors">
              <ScanSearch size={14} className={scanning ? "animate-spin" : ""} />
            </span>
            {scanning ? "Scanning…" : "Scan"}
          </button>
          <button
            aria-label="Settings"
            className="w-9 h-9 rounded-full bg-[#141414] border border-[rgba(255,255,255,0.06)] flex items-center justify-center text-[#A8A39E] hover:text-[#F5F0EB] hover:bg-[#1A1A1A] hover:border-[rgba(225,29,72,0.18)] hover:shadow-[0_0_16px_rgba(225,29,72,0.16)] hover:rotate-15 active:scale-95 transition-all"
          >
            <Settings2 size={17} strokeWidth={1.75} />
          </button>
        </div>
      </div>
    </header>
  );
}
