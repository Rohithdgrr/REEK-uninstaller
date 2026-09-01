import { Zap } from "lucide-react";
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
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      {/* Overlay rgba(10,10,10,0.85) + blur */}
      <div className="absolute inset-0 overlay-enter bg-[rgba(10,10,10,0.85)] backdrop-blur-[16px]" onClick={onCancel} aria-hidden />
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Confirm uninstall"
        className="relative w-full max-w-[560px] modal-enter bg-[#1A1A1A] rounded-[16px] border border-[rgba(225,29,72,0.2)] shadow-[0_20px_80px_rgba(0,0,0,0.8),0_0_60px_rgba(225,29,72,0.05)] overflow-hidden"
      >
        <div className="px-7 pt-7 pb-6">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-full bg-[rgba(225,29,72,0.12)] border border-[rgba(225,29,72,0.2)] flex items-center justify-center text-[#E11D48]">
              <Zap size={16} />
            </div>
            <h2 className="font-display font-semibold text-[20px] text-[#F5F0EB]">Confirm Uninstall</h2>
          </div>
          <div className="mt-4 h-px bg-[rgba(225,29,72,0.12)]" />
          <p className="mt-4 font-display italic text-[14px] text-[#A8A39E] leading-relaxed">
            This action is irreversible. Mahakali destroys to create anew.
          </p>

          <div className="mt-5 rounded-[8px] bg-[#141414] border border-[rgba(225,29,72,0.06)] p-3 max-h-[160px] overflow-auto">
            <ul className="space-y-2">
              {apps.map((a) => (
                <li key={a.id} className="flex items-center gap-2 text-[14px] text-[#F5F0EB]">
                  <span className="w-1 h-1 rounded-full bg-[#E11D48] shrink-0" />
                  <span className="truncate flex-1">{a.name}</span>
                  <span className="text-[13px] text-[#6B6661] shrink-0">{a.size_display ?? ""}</span>
                </li>
              ))}
            </ul>
          </div>

          {/* Force toggle — gold text, custom switch */}
          <label className="mt-5 flex items-center gap-3 cursor-pointer select-none">
            <button
              role="switch"
              aria-checked={force}
              onClick={() => onForceChange(!force)}
              className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors duration-200 shrink-0 ${force ? "bg-[#E11D48]" : "bg-[#2A2A2A]"}`}
            >
              <span
                className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform duration-200 ${force ? "translate-x-5" : "translate-x-1"}`}
              />
            </button>
            <span className="text-[13px] font-normal text-[#C9A84C]">Force removal if uninstaller fails</span>
          </label>
        </div>

        <div className="flex items-center justify-end gap-3 px-7 py-4 bg-[#141414] border-t border-[rgba(225,29,72,0.08)]">
          <button
            onClick={onCancel}
            className="rounded-full border border-[rgba(255,255,255,0.08)] bg-transparent px-6 py-2 text-[14px] font-medium text-[#A8A39E] hover:text-[#F5F0EB] hover:border-[rgba(255,255,255,0.15)] transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            className="inline-flex items-center gap-1.5 rounded-full bg-[#E11D48] px-6 py-2 text-[14px] font-semibold text-white shadow-[0_0_40px_rgba(225,29,72,0.3)] hover:bg-[#FF3B6A] hover:scale-[1.02] transition-all"
          >
            <Zap size={14} /> Uninstall
          </button>
        </div>
      </div>
    </div>
  );
}
