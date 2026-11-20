/**
 * Leaderboard — Loading Skeleton
 *
 * Next.js App Router automatic Suspense boundary.
 * Shows 10 shimmer rows mirroring the real leaderboard layout.
 */

export default function LeaderboardLoading() {
  return (
    <div className="min-h-screen bg-bg-primary text-text-primary">
      <div className="container mx-auto px-4 py-12">
        {/* Page header shimmer */}
        <div className="mb-10 space-y-3">
          <div className="shimmer h-3 w-36 rounded-none" />
          <div className="shimmer h-10 w-56 rounded-none" />
          <div className="shimmer h-4 w-80 rounded-none" />
          <div className="shimmer h-3 w-28 rounded-none" />
        </div>

        {/* Search placeholder */}
        <div className="mb-6">
          <div className="shimmer h-9 w-72 rounded-none" />
        </div>

        {/* Table header shimmer */}
        <div className="border border-border-color">
          <div className="bg-surface px-4 py-3 border-b border-border-color">
            <div className="shimmer h-3 w-full rounded-none" />
          </div>

          {/* 10 shimmer rows */}
          {Array.from({ length: 10 }).map((_, i) => (
            <div
              key={i}
              className="flex gap-4 px-4 py-4 border-b border-border-color last:border-b-0"
            >
              <div className="shimmer h-5 w-10 rounded-none flex-shrink-0" />
              <div className="shimmer h-5 w-40 rounded-none flex-1" />
              <div className="shimmer h-5 w-10 rounded-none" />
              <div className="hidden md:block shimmer h-5 w-8 rounded-none" />
              <div className="hidden md:block shimmer h-5 w-24 rounded-none" />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
