import { useEffect } from "react";
import { X, Check } from "lucide-react";

export function SuccessTickDialog({
  open,
  title,
  subtitle,
  details,
  onClose,
}: {
  open: boolean;
  title: string;
  subtitle?: string;
  details?: string;
  onClose: () => void;
}) {
  useEffect(() => {
    if (open) {
      const t = setTimeout(onClose, 2800);
      return () => clearTimeout(t);
    }
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[80] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/70 backdrop-blur-md" onClick={onClose} />
      <div className="relative bg-[#141414] border border-[rgba(17,255,153,0.18)] rounded-[24px] p-8 md:p-10 max-w-[420px] w-full text-center shadow-[0_24px_80px_rgba(0,0,0,0.8),0_0_40px_rgba(17,255,153,0.12)] overflow-hidden animate-[scaleIn_320ms_cubic-bezier(0.34,1.56,0.64,1)]">
        {/* subtle success glow */}
        <div className="absolute -top-10 left-1/2 -translate-x-1/2 w-[280px] h-[160px] bg-[radial-gradient(ellipse_at_center,rgba(17,255,153,0.18),transparent_70%)] pointer-events-none" />
        {/* confetti dots */}
        <div className="absolute inset-0 pointer-events-none overflow-hidden">
          <span className="absolute top-[18%] left-[12%] w-1.5 h-1.5 bg-[#11FF99] rounded-full animate-[confetti_900ms_ease_out_200ms_both]" />
          <span className="absolute top-[22%] right-[14%] w-1 h-1 bg-[#E11D48] rounded-full animate-[confetti_900ms_ease_out_400ms_both]" />
          <span className="absolute top-[30%] left-[8%] w-1 h-1 bg-[#C9A84C] rounded-full animate-[confetti_900ms_ease_out_600ms_both]" />
          <span className="absolute top-[28%] right-[10%] w-1.5 h-1.5 bg-[#11FF99] rounded-full animate-[confetti_900ms_ease_out_300ms_both]" />
        </div>

        <button onClick={onClose} className="absolute top-3 right-3 w-8 h-8 rounded-full bg-black border border-white/10 flex items-center justify-center text-white/60 hover:text-white">
          <X size={14} />
        </button>

        {/* Green tick circle — UPI style */}
        <div className="mx-auto w-[96px] h-[96px] rounded-full bg-[#11FF99] flex items-center justify-center shadow-[0_0_30px_rgba(17,255,153,0.5),0_0_60px_rgba(17,255,153,0.2)] animate-[tickPop_520ms_cubic-bezier(0.34,1.56,0.64,1)_100ms_both]">
          <Check size={52} strokeWidth={3.5} className="text-black animate-[tickDraw_480ms_ease-out_320ms_both]" />
          {/* ripple */}
          <span className="absolute w-[96px] h-[96px] rounded-full border-2 border-[#11FF99]/40 animate-[ripple_900ms_ease-out_400ms_both]" />
          <span className="absolute w-[96px] h-[96px] rounded-full border border-[#11FF99]/20 animate-[ripple_900ms_ease-out_600ms_both]" />
        </div>

        <h3 className="mt-6 text-[20px] font-bold text-white tracking-tight">{title}</h3>
        {subtitle && <p className="mt-1.5 text-[13px] text-[#A8A39E] leading-relaxed">{subtitle}</p>}
        {details && <p className="mt-3 inline-flex px-3 py-1.5 rounded-full bg-[rgba(17,255,153,0.1)] border border-[rgba(17,255,153,0.18)] text-[11px] font-medium text-[#11FF99]">{details}</p>}

        <div className="mt-6 flex justify-center">
          <div className="h-1 w-24 rounded-full bg-[#1A1A1A] overflow-hidden">
            <div className="h-full bg-[#11FF99] animate-[shrink_2800ms_linear_both]" />
          </div>
        </div>

        <style>{`
          @keyframes scaleIn { from { opacity:0; transform:scale(0.85) translateY(8px)} to { opacity:1; transform:scale(1) translateY(0)} }
          @keyframes tickPop { 0% { transform:scale(0.4)} 60% { transform:scale(1.08)} 100% { transform:scale(1)} }
          @keyframes tickDraw { from { stroke-dasharray: 0 100; opacity:0 } to { stroke-dasharray: 100 100; opacity:1 } }
          @keyframes ripple { from { transform:scale(1); opacity:0.6} to { transform:scale(1.55); opacity:0} }
          @keyframes confetti { 0% { transform:translateY(-8px) scale(0); opacity:0} 50% { opacity:1} 100% { transform:translateY(18px) scale(1); opacity:0} }
          @keyframes shrink { from { width:100% } to { width:0% } }
        `}</style>
      </div>
    </div>
  );
}
