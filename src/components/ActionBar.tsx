export function ActionBar({
  count,
  onUninstall,
  onClear,
}: {
  count: number;
  onUninstall: () => void;
  onClear: () => void;
}) {
  return (
    <div className="fixed bottom-0 left-0 right-0 z-30 bg-[rgba(10,10,10,0.6)] backdrop-blur-[16px] border-t border-[rgba(225,29,72,0.12)]">
      <div className="mx-auto max-w-[1200px] px-6 h-[72px] flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <span className="text-[14px] font-medium text-[#A8A39E]">
            <span className="text-[#E11D48] [text-shadow:0_0_20px_rgba(225,29,72,0.3)]">{count}</span>
            <span className="ml-1">applications selected</span>
          </span>
          {count > 0 && (
            <button onClick={onClear} className="text-[13px] font-normal text-[#6B6661] hover:text-[#F5F0EB] hover:underline underline-offset-2 transition-colors">
              Deselect All
            </button>
          )}
        </div>
        <button
          onClick={onUninstall}
          disabled={count === 0}
          aria-label="Uninstall selected"
          className={`inline-flex items-center justify-center rounded-full px-8 py-[10px] text-[14px] font-semibold text-white transition-all duration-200
            ${count === 0
              ? "bg-[#E11D48] opacity-40 cursor-not-allowed shadow-none"
              : "bg-[#E11D48] shadow-[0_0_40px_rgba(225,29,72,0.3)] hover:bg-[#FF3B6A] hover:scale-[1.03] btn-breathe"
            }`}
        >
          Uninstall
        </button>
      </div>
    </div>
  );
}
