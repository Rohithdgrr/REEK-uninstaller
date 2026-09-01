export function Toast({ message, onClose }: { message: string; onClose: () => void }) {
  if (!message) return null;
  return (
    <div className="fixed top-4 right-4 z-50 bg-slate-900 text-white text-sm px-4 py-3 rounded-xl shadow-lg flex items-center gap-3">
      <span>{message}</span>
      <button onClick={onClose} aria-label="Dismiss" className="text-slate-300 hover:text-white">✕</button>
    </div>
  );
}
