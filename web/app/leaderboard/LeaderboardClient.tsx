/**
 * LeaderboardClient
 *
 * Client component that fetches leaderboard data, manages pagination, and
 * runs an address search that queries the *full* leaderboard (every page),
 * not just the rows currently loaded in the table.
 */

"use client";

import { useState, useEffect, useCallback } from "react";
import { LeaderboardTable, type LeaderboardEntry } from "@/components/leaderboard";
import { Pagination } from "@/components/ui/Pagination";
import { LEADERBOARD_PAGE_SIZE } from "@/lib/constants";

interface LeaderboardResponse {
  entries: LeaderboardEntry[];
  total: number;
  page: number;
}

/** Wait time after the last keystroke before a search query is fired. */
const SEARCH_DEBOUNCE_MS = 300;

/** Safety cap — a single search pages through at most this many pages. */
const SEARCH_MAX_PAGES = 40;

/**
 * Query the leaderboard API for *every* page and return the entries whose
 * address contains `query` (case-insensitive).
 *
 * Paging the API (instead of filtering the rows already in the table) is
 * what makes the search global: an address ranked on page 3+ is still found
 * while the table itself is displaying page 1.
 */
async function searchFullLeaderboard(
  query: string,
  signal: AbortSignal
): Promise<LeaderboardEntry[]> {
  const needle = query.toLowerCase();
  const matches: LeaderboardEntry[] = [];
  let currentPage = 1;
  let totalPages = 1;

  while (currentPage <= totalPages && currentPage <= SEARCH_MAX_PAGES) {
    // Same-origin proxy — never hit NEXT_PUBLIC_API_URL from the browser.
    const res = await fetch(
      `/api/leaderboard?page=${currentPage}&limit=${LEADERBOARD_PAGE_SIZE}`,
      { signal }
    );
    if (!res.ok) throw new Error(`HTTP ${res.status}`);

    const data: LeaderboardResponse = await res.json();
    const rows = data.entries ?? [];
    totalPages = Math.max(1, Math.ceil((data.total ?? 0) / LEADERBOARD_PAGE_SIZE));

    for (const entry of rows) {
      if (entry.address.toLowerCase().includes(needle)) matches.push(entry);
    }
    if (rows.length === 0) break;
    currentPage += 1;
  }

  return matches;
}

interface SearchState {
  /** The query these results belong to — stale results are never shown. */
  query: string;
  results: LeaderboardEntry[];
  failed: boolean;
}

export default function LeaderboardClient() {
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [isLoading, setIsLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [activeQuery, setActiveQuery] = useState("");
  const [searchState, setSearchState] = useState<SearchState | null>(null);

  const totalPages = Math.max(1, Math.ceil(total / LEADERBOARD_PAGE_SIZE));
  const isSearching = activeQuery.length > 0;

  // Results are only valid for the query they were produced for; anything
  // else means the search for `activeQuery` is still in flight.
  const currentSearch =
    searchState && searchState.query === activeQuery ? searchState : null;
  const isSearchPending = isSearching && currentSearch === null;
  const searchResults = currentSearch?.results ?? [];
  const searchFailed = currentSearch?.failed ?? false;

  const fetchLeaderboard = useCallback(async (currentPage: number) => {
    setIsLoading(true);
    try {
      // Same-origin proxy — never hit NEXT_PUBLIC_API_URL from the browser.
      const res = await fetch(
        `/api/leaderboard?page=${currentPage}&limit=${LEADERBOARD_PAGE_SIZE}`,
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data: LeaderboardResponse = await res.json();
      setEntries(data.entries ?? []);
      setTotal(data.total ?? 0);
    } catch {
      setEntries([]);
      setTotal(0);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchLeaderboard(page);
  }, [page, fetchLeaderboard]);

  // Debounce keystrokes so the full-leaderboard query isn't fired per key.
  useEffect(() => {
    const handle = setTimeout(
      () => setActiveQuery(searchQuery.trim()),
      SEARCH_DEBOUNCE_MS
    );
    return () => clearTimeout(handle);
  }, [searchQuery]);

  // Dedicated search across the whole leaderboard whenever the query changes.
  useEffect(() => {
    if (!activeQuery) return;

    const controller = new AbortController();
    searchFullLeaderboard(activeQuery, controller.signal)
      .then((results) => {
        if (controller.signal.aborted) return;
        setSearchState({ query: activeQuery, results, failed: false });
      })
      .catch((err: unknown) => {
        if (controller.signal.aborted) return;
        if (err instanceof Error && err.name === "AbortError") return;
        setSearchState({ query: activeQuery, results: [], failed: true });
      });

    return () => controller.abort();
  }, [activeQuery]);

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    window.scrollTo({ top: 0, behavior: "smooth" });
  };

  const clearSearch = () => {
    setSearchQuery("");
    setActiveQuery("");
  };

  // Search results span every page, so they replace the paginated rows.
  const displayedEntries = isSearching ? searchResults : entries;
  const showEmptySearch =
    isSearching && !isSearchPending && !searchFailed && searchResults.length === 0;

  return (
    <div className="min-h-screen bg-bg-primary text-text-primary">
      <div className="container mx-auto px-4 py-12">
        {/* ── Page Header ─────────────────────────────────────────────── */}
        <div className="mb-10 space-y-3">
          <p className="font-mono text-xs uppercase tracking-widest text-text-muted">
            Stellar Contributors
          </p>
          <h1 className="font-orbitron text-4xl font-bold uppercase tracking-wider text-text-primary">
            Leaderboard
          </h1>
          <p className="font-mono text-sm text-text-muted max-w-xl">
            Top contributors ranked by reputation score earned through completed
            milestones.
          </p>
          {total > 0 && (
            <p className="font-mono text-xs text-accent-primary">
              {total.toLocaleString()} total contributor
              {total !== 1 ? "s" : ""}
            </p>
          )}
        </div>

        {/* ── Search / Filter ──────────────────────────────────────────── */}
        <div className="mb-6">
          <input
            id="leaderboard-search"
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search the full leaderboard by address…"
            aria-label="Search the full leaderboard by address"
            className="w-full max-w-sm bg-surface border border-border-color px-4 py-2 font-mono text-sm text-text-primary placeholder:text-text-muted focus:border-accent-primary focus:outline-none rounded-none"
          />
        </div>

        {/* ── Search status ────────────────────────────────────────────── */}
        {isSearching && (
          <div
            className="mb-4 flex flex-wrap items-center justify-between gap-3"
            role="status"
            aria-live="polite"
          >
            <p className="font-mono text-xs text-text-muted">
              {isSearchPending
                ? `Searching the full leaderboard for “${activeQuery}”…`
                : searchFailed
                  ? `Search for “${activeQuery}” failed — please try again.`
                  : `Showing ${searchResults.length} match${
                      searchResults.length === 1 ? "" : "es"
                    } across the full leaderboard.`}
            </p>
            <button
              type="button"
              onClick={clearSearch}
              className="font-mono text-xs uppercase tracking-widest text-accent-primary hover:underline"
            >
              Clear search
            </button>
          </div>
        )}

        {/* ── Table ────────────────────────────────────────────────────── */}
        {showEmptySearch ? (
          <div className="border border-border-color bg-surface px-4 py-12 text-center font-mono text-sm text-text-muted">
            No contributor address matches “{activeQuery}” anywhere on the
            leaderboard.
          </div>
        ) : searchFailed ? (
          <div className="border border-danger/40 bg-surface px-4 py-12 text-center font-mono text-sm text-text-muted">
            The leaderboard search could not be completed. Clear the search to
            browse the leaderboard as normal.
          </div>
        ) : (
          <LeaderboardTable
            entries={displayedEntries}
            isLoading={isLoading || isSearchPending}
          />
        )}

        {/* ── Pagination (browse mode only — search results span all pages) */}
        {!isLoading && !isSearching && (
          <Pagination
            page={page}
            totalPages={totalPages}
            onPageChange={handlePageChange}
          />
        )}
      </div>
    </div>
  );
}
