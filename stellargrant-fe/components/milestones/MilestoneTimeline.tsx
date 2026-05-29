"use client";

/**
 * MilestoneTimeline — vertical timeline for grant detail page.
 */

import Link from "next/link";
import type { Milestone } from "@/types";

interface MilestoneTimelineProps {
  milestones: Milestone[];
  grantId: string;
}

function statusLabel(m: Milestone): string {
  if (m.paid) return "Paid";
  if (m.approved) return "Approved";
  if (m.submitted) return "Submitted";
  if (m.overdue) return "Overdue";
  if ((m.daysUntilDeadline ?? Infinity) <= 7) return "Due Soon";
  return "Pending";
}

function statusClass(label: string): string {
  switch (label) {
    case "Paid":
      return "bg-success/20 text-success border-success/40";
    case "Approved":
      return "bg-accent-secondary/20 text-accent-secondary border-accent-secondary/40";
    case "Submitted":
      return "bg-warning/20 text-warning border-warning/40";
    case "Overdue":
      return "bg-danger/20 text-danger border-danger/40";
    case "Due Soon":
      return "bg-warning/15 text-warning border-warning/30";
    default:
      return "bg-surface text-text-muted border-border-color";
  }
}

export function MilestoneTimeline({ milestones, grantId }: MilestoneTimelineProps) {
  if (milestones.length === 0) {
    return (
      <p className="font-mono text-sm text-text-muted">No milestones defined yet.</p>
    );
  }

  return (
    <ol className="relative border-l border-border-color ml-3 space-y-6">
      {milestones.map((m) => {
        const label = statusLabel(m);
        return (
          <li key={m.idx} className="ml-6">
            <span
              className="absolute -left-[7px] mt-1.5 h-3 w-3 rounded-full border-2 border-accent-primary bg-bg-primary"
              aria-hidden
            />
            <Link
              href={`/grants/${grantId}/milestones/${m.idx}`}
              className="block rounded-none border border-border-color bg-surface p-4 ring-1 ring-border-color hover:border-accent-secondary/50 transition-colors"
            >
              <div className="flex flex-wrap items-center justify-between gap-2 mb-1">
                <span className="font-orbitron text-sm text-text-primary">
                  M{m.idx + 1}: {m.title}
                </span>
                <span
                  className={`font-mono text-[10px] uppercase tracking-wider px-2 py-0.5 border ${statusClass(label)}`}
                >
                  {label}
                </span>
              </div>
              {m.description && (
                <p className="font-mono text-xs text-text-muted line-clamp-2">
                  {m.description}
                </p>
              )}
            </Link>
          </li>
        );
      })}
    </ol>
  );
}
