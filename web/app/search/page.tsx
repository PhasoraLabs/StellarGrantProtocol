import type { Metadata } from "next";
import { Suspense } from "react";
import { SearchPageClient } from "./SearchPageClient";

export const metadata: Metadata = {
  title: "Search — StellarGrant Protocol",
};

function SearchFallback() {
  return (
    <div className="container mx-auto max-w-6xl px-4 py-8">
      <div className="shimmer mb-6 h-12 rounded-none" />
      <div className="shimmer h-48 rounded-none" />
    </div>
  );
}

export default function SearchPage() {
  return (
    <Suspense fallback={<SearchFallback />}>
      <SearchPageClient />
    </Suspense>
  );
}
