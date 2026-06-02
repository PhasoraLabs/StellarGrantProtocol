/**
 * LeaderboardTable
 *
 * Displays ranked contributor entries with medal styling for the top 3,
 * colour-coded reputation scores, formatted token amounts, and shimmer
 * loading skeletons. Responsive: on mobile (<768 px) only Rank, Contributor
 * and Reputation columns are shown.
 */

"use client";

import React from "react";
import { WalletAddress } from "@/components/wallet/WalletAddress";
import { formatTokenAmount } from "@/lib/tokens";

export interface LeaderboardEntry {
  rank: number;
  address: string;
  reputationScore: number;
  grantsCompleted: number;
  totalEarned: bigint;
  token: string;
}

export interface LeaderboardTableProps {
  entries: LeaderboardEntry[];
  isLoading?: boolean;
}

// ── Medal helpers ─────────────────────────────────────────────────────────────

const MEDALS: Record<number, string> = { 1: "🥇", 2: "🥈", 3: "🥉" };

function RankCell({ rank }: { rank: number }) {
  if (rank <= 3) {
    return (
      <span className="inline-flex items-center gap-1 font-mono text-sm font-bold">
        {MEDALS[rank]}
        <span className="text-text-muted">{rank}</span>
      </span>
    );
  }
  return (
    <span className="font-mono text-sm text-text-muted tabular-nums">{rank}</span>
  );
}

// ── Reputation colour coding ──────────────────────────────────────────────────

function reputationColor(score: number): string {
  if (score >= 90) return "text-success";
  if (score >= 70) return "text-text-primary";
  if (score >= 50) return "text-warning";
  return "text-danger";
}

// ── Row styling by rank ───────────────────────────────────────────────────────

function rowClass(rank: number): string {
  switch (rank) {
    case 1:
      return "border-l-2 border-accent-primary bg-accent-primary/5";
    case 2:
      return "border-l-2 border-text-muted/50 bg-surface/50";
    case 3:
      return "border-l-2 border-warning/50 bg-warning/5";
    default:
      return "border-l-2 border-transparent bg-transparent hover:bg-surface/30";
  }
}

// ── Token amount formatting ───────────────────────────────────────────────────

function formatEarned(amount: bigint, token: string): string {
  const isNative = token === "native" || token === "XLM";
  const decimals = isNative ? 7 : 6; // XLM=7, USDC=6
  const symbol = isNative ? "XLM" : "USDC";
  return formatTokenAmount(amount, decimals, { symbol, showSymbol: true });
}

// ── Shimmer rows (loading state) ─────────────────────────────────────────────

function ShimmerRows() {
  return (
    <>
      {Array.from({ length: 10 }).map((_, i) => (
        <tr key={i}>
          <td className="px-4 py-3">
            <div className="shimmer h-4 w-8 rounded-none" />
          </td>
          <td className="px-4 py-3">
            <div className="shimmer h-4 w-32 rounded-none" />
          </td>
          <td className="px-4 py-3">
            <div className="shimmer h-4 w-10 rounded-none" />
          </td>
          <td className="hidden md:table-cell px-4 py-3">
            <div className="shimmer h-4 w-6 rounded-none" />
          </td>
          <td className="hidden md:table-cell px-4 py-3">
            <div className="shimmer h-4 w-20 rounded-none" />
          </td>
        </tr>
      ))}
    </>
  );
}

// ── Main component ────────────────────────────────────────────────────────────

export function LeaderboardTable({ entries, isLoading = false }: LeaderboardTableProps) {
  const isEmpty = !isLoading && entries.length === 0;

  return (
    <div className="w-full overflow-x-auto border border-border-color">
      <table className="w-full border-collapse text-left text-sm">
        <thead>
          <tr className="border-b border-border-color bg-surface">
            <th className="px-4 py-3 font-mono text-xs uppercase tracking-widest text-text-muted w-16">
              Rank
            </th>
            <th className="px-4 py-3 font-mono text-xs uppercase tracking-widest text-text-muted">
              Contributor
            </th>
            <th className="px-4 py-3 font-mono text-xs uppercase tracking-widest text-text-muted">
              Reputation
            </th>
            <th className="hidden md:table-cell px-4 py-3 font-mono text-xs uppercase tracking-widest text-text-muted">
              Grants Completed
            </th>
            <th className="hidden md:table-cell px-4 py-3 font-mono text-xs uppercase tracking-widest text-text-muted">
              Total Earned
            </th>
          </tr>
        </thead>
        <tbody className="divide-y divide-border-color">
          {isLoading ? (
            <ShimmerRows />
          ) : isEmpty ? (
            <tr>
              <td
                colSpan={5}
                className="px-4 py-12 text-center font-mono text-sm text-text-muted"
              >
                No contributors yet — be the first to complete a milestone.
              </td>
            </tr>
          ) : (
            entries.map((entry) => (
              <tr
                key={entry.address}
                className={`transition-colors duration-150 ${rowClass(entry.rank)}`}
              >
                {/* Rank */}
                <td className="px-4 py-3">
                  <RankCell rank={entry.rank} />
                </td>

                {/* Contributor */}
                <td className="px-4 py-3">
                  <WalletAddress address={entry.address} showCopyIcon />
                </td>

                {/* Reputation */}
                <td className="px-4 py-3">
                  <span
                    className={`font-orbitron text-sm font-bold tabular-nums ${reputationColor(
                      entry.reputationScore
                    )}`}
                  >
                    {entry.reputationScore}
                  </span>
                </td>

                {/* Grants Completed — hidden on mobile */}
                <td className="hidden md:table-cell px-4 py-3 font-mono text-sm text-text-primary tabular-nums">
                  {entry.grantsCompleted}
                </td>

                {/* Total Earned — hidden on mobile */}
                <td className="hidden md:table-cell px-4 py-3">
                  <span className="font-orbitron text-sm text-text-primary tabular-nums">
                    {formatEarned(entry.totalEarned, entry.token)}
                  </span>
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}
