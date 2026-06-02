/**
 * LeaderboardClient
 *
 * Client component that fetches leaderboard data, manages pagination,
 * client-side address search, and renders the LeaderboardTable.
 */

"use client";

import { useState, useEffect, useCallback } from "react";
import { LeaderboardTable, type LeaderboardEntry } from "@/components/leaderboard";
import { Pagination } from "@/components/ui/Pagination";
import { API_URL, LEADERBOARD_PAGE_SIZE } from "@/lib/constants";

interface LeaderboardResponse {
  entries: LeaderboardEntry[];
  total: number;
  page: number;
}

export default function LeaderboardClient() {
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [isLoading, setIsLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");

  const totalPages = Math.max(1, Math.ceil(total / LEADERBOARD_PAGE_SIZE));

  const fetchLeaderboard = useCallback(async (currentPage: number) => {
    setIsLoading(true);
    try {
      const res = await fetch(
        `${API_URL}/leaderboard?page=${currentPage}&limit=${LEADERBOARD_PAGE_SIZE}`
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

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    window.scrollTo({ top: 0, behavior: "smooth" });
  };

  // Client-side filter on current page data
  const filtered = searchQuery
    ? entries.filter((e) =>
        e.address.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : entries;

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
            placeholder="Filter by address…"
            aria-label="Filter contributors by address"
            className="w-full max-w-sm bg-surface border border-border-color px-4 py-2 font-mono text-sm text-text-primary placeholder:text-text-muted focus:border-accent-primary focus:outline-none rounded-none"
          />
        </div>

        {/* ── Table ────────────────────────────────────────────────────── */}
        <LeaderboardTable entries={filtered} isLoading={isLoading} />

        {/* ── Pagination ───────────────────────────────────────────────── */}
        {!isLoading && (
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
