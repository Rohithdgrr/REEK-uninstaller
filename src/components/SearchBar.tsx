import { Search } from "lucide-react";

export function SearchBar({
  value,
  onChange,
  count,
}: {
  value: string;
  onChange: (v: string) => void;
  count: number;
}) {
  return (
    <div className="h-[56px] flex items-center">
      <div className="relative flex-1 max-w-[560px] mx-auto w-full">
        <Search size={16} className="absolute left-5 top-1/2 -translate-y-1/2 text-[#6B6661] pointer-events-none" aria-hidden />
        <input
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="Search applications..."
          aria-label="Search applications"
          className="w-full rounded-full border border-[rgba(225,29,72,0.08)] bg-[#141414] pl-11 pr-[88px] py-2 text-[14px] font-normal text-[#F5F0EB] placeholder:text-[#6B6661] focus:outline-none focus:border-[rgba(225,29,72,0.25)] focus:ring-[2px] focus:ring-[rgba(225,29,72,0.25)] transition"
          style={{ paddingTop: '8px', paddingBottom: '8px' }}
        />
        <span className="absolute right-5 top-1/2 -translate-y-1/2 text-[14px] font-medium pointer-events-none">
          <span className="text-[#C9A84C]">{count}</span>
          <span className="text-[#A8A39E]"> apps</span>
        </span>
      </div>
    </div>
  );
}
