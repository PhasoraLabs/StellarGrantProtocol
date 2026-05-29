/**
 * Grant Detail — Loading Skeleton (two-column layout)
 */

export default function GrantDetailLoading() {
  return (
    <div className="container mx-auto max-w-6xl px-4 py-8">
      <div className="shimmer h-4 w-48 rounded-none mb-4" />
      <div className="flex flex-wrap gap-4 mb-6">
        <div className="shimmer h-10 flex-1 min-w-[200px] rounded-none" />
        <div className="shimmer h-8 w-24 rounded-none" />
      </div>
      <div className="shimmer h-4 w-64 rounded-none mb-8" />
      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3 mb-8">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="shimmer h-16 rounded-none" />
        ))}
      </div>
      <div className="flex flex-col lg:flex-row gap-8">
        <div className="flex-1 space-y-6 order-2 lg:order-1">
          <div className="shimmer h-40 rounded-none" />
          <div className="shimmer h-6 w-32 rounded-none" />
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="shimmer h-20 rounded-none" />
          ))}
        </div>
        <div className="w-full lg:w-80 space-y-4 shrink-0 order-1 lg:order-2">
          <div className="shimmer h-48 rounded-none" />
          <div className="shimmer h-32 rounded-none" />
        </div>
      </div>
    </div>
  );
}
