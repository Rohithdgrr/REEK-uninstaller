export function Toast({ message, onClose }: { message: string; onClose: () => void }) {
  if (!message) return null;
  return (
    <div className="fixed top-4 right-4 z-50 bg-[#0a0a0c] border border-[rgba(255,255,255,0.14)] text-[#fcfdff] text-sm px-4 py-3 rounded-[12px] shadow-2xl flex items-center gap-3">
      <span>{message}</span>
      <button onClick={onClose} aria-label="Dismiss" className="text-[rgba(252,253,255,0.6)] hover:text-[#fcfdff]">✕</button>
    </div>
  );
}
