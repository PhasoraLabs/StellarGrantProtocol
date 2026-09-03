/**
 * Grants List — Loading Skeleton
 *
 * Next.js App Router automatic Suspense boundary.
 * Rendered instantly while the GrantsPage component streams in.
 * Mirrors the real page layout: header + filter bar + 6-card grid.
 */

export default function GrantsLoading() {
  return (
    <div className="container mx-auto px-4 py-8">
      {/* Page header */}
      <div className="mb-8 space-y-3">
        <div className="shimmer h-3 w-28 rounded-none" />
        <div className="shimmer h-8 w-36 rounded-none" />
        <div className="shimmer h-4 w-80 rounded-none" />
      </div>

      {/* Filter bar — two shimmer pill chips */}
      <div className="mb-6 flex flex-wrap gap-3">
        <div className="shimmer h-8 w-24 rounded-none" />
        <div className="shimmer h-8 w-28 rounded-none" />
      </div>

      {/* Grant card grid: 1 col mobile → 2 col tablet → 3 col desktop */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="shimmer h-52 rounded-none" />
        ))}
      </div>
    </div>
  );
}
