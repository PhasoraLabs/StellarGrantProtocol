"use client";

/**
 * GrantStats Component
 *
 * Horizontal row / 2×3 mobile grid of aggregate stat tiles for a single grant.
 * Each tile renders a muted IBM Plex Mono label above an Orbitron value.
 *
 * Deadline tile colour:
 *   - default     — normal text
 *   - ≤ 7 days    — warning (amber)
 *   - overdue / 0 — danger (red)
 */

import { useMemo } from "react";
import { formatTokenAmount, getDefaultDecimals } from "@/lib/tokens";

// ── Props ─────────────────────────────────────────────────────────────────────

export interface GrantStatsProps {
  totalBudget: bigint;
  fundedAmount: bigint;
  milestoneCount: number;
  completedMilestones: number;
  reviewerCount: number;
  token: string;
  /** Unix timestamp (seconds). Omit to hide the deadline tile. */
  deadline?: bigint;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function percentFunded(funded: bigint, budget: bigint): number {
  if (budget === 0n) return 0;
  return Math.min(100, Math.round(Number((funded * 10000n) / budget) / 100));
}

type DeadlineState = "ok" | "warning" | "danger";

function deadlineState(deadlineTs: bigint): DeadlineState {
  const nowMs = Date.now();
  const deadlineMs = Number(deadlineTs) * 1000;
  const daysLeft = (deadlineMs - nowMs) / (1000 * 60 * 60 * 24);
  if (daysLeft <= 0) return "danger";
  if (daysLeft <= 7) return "warning";
  return "ok";
}

function formatDeadlineDate(deadlineTs: bigint): string {
  return new Date(Number(deadlineTs) * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function deriveStatus(
  funded: bigint,
  budget: bigint,
  completed: number,
  total: number
): string {
  if (budget > 0n && funded >= budget && completed === total && total > 0)
    return "Complete";
  if (funded === 0n) return "Unfunded";
  if (funded >= budget) return "Funded";
  if (completed > 0) return "In Progress";
  return "Open";
}

// ── Tile ──────────────────────────────────────────────────────────────────────

interface TileProps {
  label: string;
  value: string;
  valueColour?: string; // tailwind text-* class
}

function Tile({ label, value, valueColour = "text-foreground" }: TileProps) {
  return (
    <div
      className="flex flex-col gap-1 rounded-sm border px-4 py-3"
      style={{ background: "#111D35", borderColor: "#1E3A5F" }}
    >
      <span
        className="font-mono text-[10px] uppercase tracking-[0.24em] text-text-muted"
        style={{ fontFamily: "'IBM Plex Mono', monospace" }}
      >
        {label}
      </span>
      <span
        className={`text-base font-semibold leading-snug ${valueColour}`}
        style={{ fontFamily: "'Orbitron', sans-serif" }}
      >
        {value}
      </span>
    </div>
  );
}

// ── Component ─────────────────────────────────────────────────────────────────

export function GrantStats({
  totalBudget,
  fundedAmount,
  milestoneCount,
  completedMilestones,
  reviewerCount,
  token,
  deadline,
}: GrantStatsProps) {
  const decimals = getDefaultDecimals(token);
  const pct = useMemo(
    () => percentFunded(fundedAmount, totalBudget),
    [fundedAmount, totalBudget]
  );

  const budgetStr = formatTokenAmount(totalBudget, decimals, {
    symbol: token,
    showSymbol: true,
  });

  const fundedStr = `${formatTokenAmount(fundedAmount, decimals, {
    symbol: token,
    showSymbol: true,
  })} — ${pct}%`;

  const milestonesStr =
    milestoneCount > 0
      ? `${completedMilestones} / ${milestoneCount} complete`
      : "—";

  const statusStr = deriveStatus(
    fundedAmount,
    totalBudget,
    completedMilestones,
    milestoneCount
  );

  // Deadline tile
  let deadlineStr = "—";
  let deadlineColour = "text-foreground";
  if (deadline !== undefined && deadline > 0n) {
    deadlineStr = formatDeadlineDate(deadline);
    const state = deadlineState(deadline);
    if (state === "warning") deadlineColour = "text-warning";
    if (state === "danger") deadlineColour = "text-danger";
  }

  return (
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
      <Tile label="Budget" value={budgetStr} />
      <Tile label="Funded" value={fundedStr} />
      <Tile label="Milestones" value={milestonesStr} />
      <Tile label="Reviewers" value={String(reviewerCount)} />
      <Tile
        label="Deadline"
        value={deadlineStr}
        valueColour={deadlineColour}
      />
      <Tile label="Status" value={statusStr} />
    </div>
  );
}
