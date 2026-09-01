export function Skeleton() {
  return (
    <div className="bg-[#141414] rounded-[16px] border border-[rgba(225,29,72,0.06)] p-6 space-y-3 animate-pulse flex flex-col justify-center w-full">
      <div className="h-6 bg-[rgba(225,29,72,0.08)] rounded-full w-1/3" />
      <div className="space-y-2 flex-1 flex flex-col justify-center">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="h-[48px] bg-[#1A1A1A] rounded-[8px] border border-[rgba(225,29,72,0.04)] flex-1 min-h-[48px] max-h-[48px]" />
        ))}
      </div>
    </div>
  );
}
