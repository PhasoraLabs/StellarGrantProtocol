"use client";

import { motion } from "framer-motion";

const SEGMENT_COLORS = [
  "bg-accent-primary",
  "bg-accent-secondary",
  "bg-teal-500",
  "bg-purple-500",
  "bg-pink-500",
  "bg-orange-500",
  "bg-cyan-500",
  "bg-emerald-500",
] as const;

export interface BudgetDistributionChartProps {
  totalBudget: number;
  milestones: Array<{
    title: string;
    reward: number;
  }>;
  token: string;
}

export function BudgetDistributionChart({
  totalBudget,
  milestones,
  token,
}: BudgetDistributionChartProps) {
  const sum = milestones.reduce((acc, m) => acc + (Number(m.reward) || 0), 0);
  const budget = Math.max(totalBudget, 0);
  const unallocated = Math.max(0, budget - sum);
  const overBy = Math.max(0, sum - budget);
  const isOver = sum > budget && budget > 0;
  const isExact = budget > 0 && sum === budget;

  const segments = milestones.map((m, i) => ({
    ...m,
    reward: Number(m.reward) || 0,
    color: SEGMENT_COLORS[i % SEGMENT_COLORS.length],
  }));

  return (
    <div className="space-y-3">
      <div className="flex justify-between font-mono text-xs text-text-muted">
        <span>Total: {budget.toLocaleString()} {token}</span>
        {isOver && (
          <span className="text-danger font-orbitron text-[10px] uppercase tracking-wider">
            ⚠️ OVER BUDGET
          </span>
        )}
      </div>

      <div
        className={[
          "relative flex h-8 w-full overflow-hidden border border-border-color",
          isOver ? "bg-danger/30" : "bg-bg-secondary",
        ].join(" ")}
      >
        {budget <= 0 ? (
          <div className="w-full h-full border border-dashed border-border-color" />
        ) : (
          <>
            {segments.map((seg, i) => {
              if (seg.reward <= 0) return null;
              const widthPct = (seg.reward / budget) * 100;
              return (
                <motion.div
                  key={`${seg.title}-${i}`}
                  className={`h-full ${seg.color} shrink-0`}
                  initial={false}
                  animate={{ width: `${widthPct}%` }}
                  transition={{ duration: 0.3, ease: "easeOut" }}
                  title={`${seg.title}: ${seg.reward} ${token}`}
                />
              );
            })}
            {!isOver && unallocated > 0 && (
              <motion.div
                className="h-full shrink-0 border border-dashed border-border-color bg-border-color/40"
                initial={false}
                animate={{ width: `${(unallocated / budget) * 100}%` }}
                transition={{ duration: 0.3, ease: "easeOut" }}
                title={`Unallocated: ${unallocated} ${token}`}
              />
            )}
            {isOver && (
              <motion.div
                className="h-full shrink-0 bg-danger"
                initial={false}
                animate={{ width: `${Math.min(100, (overBy / budget) * 100)}%` }}
                transition={{ duration: 0.3, ease: "easeOut" }}
              />
            )}
          </>
        )}
      </div>

      <ul className="flex flex-wrap gap-x-4 gap-y-2">
        {segments.map((seg, i) => {
          const pct = budget > 0 ? Math.round((seg.reward / budget) * 100) : 0;
          return (
            <li key={`legend-${i}`} className="flex items-center gap-2 font-mono text-xs text-text-muted">
              <span className={`inline-block h-2 w-2 rounded-full ${seg.color}`} />
              M{i + 1}: {seg.title || `Phase ${i + 1}`}{" "}
              <span className="text-text-primary">
                {seg.reward.toLocaleString()} {token} ({pct}%)
              </span>
            </li>
          );
        })}
        {!isOver && unallocated > 0 && (
          <li className="flex items-center gap-2 font-mono text-xs text-text-muted">
            <span className="inline-block h-2 w-2 border border-dashed border-border-color bg-border-color/40" />
            Unallocated: {unallocated.toLocaleString()} {token}
          </li>
        )}
      </ul>

      <p
        className={[
          "font-mono text-xs",
          isExact
            ? "text-success"
            : isOver
              ? "text-danger"
              : "text-warning",
        ].join(" ")}
      >
        {isExact && "✓ Budget fully allocated"}
        {!isExact && !isOver && budget > 0 && `⚠ ${unallocated.toLocaleString()} ${token} unallocated`}
        {isOver && `✗ Over budget by ${overBy.toLocaleString()} ${token}`}
        {budget <= 0 && "Set a total budget to allocate milestones"}
      </p>
    </div>
  );
}
