/**
 * MilestoneList Component
 *
 * Ordered list of milestones with status indicators.
 * Displays token and payout amount for each milestone.
 * Clicking a milestone navigates to its detail page.
 * Uses design-system tokens exclusively — no generic Tailwind colours.
 */

"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { formatTokenAmount, getTokenMetadata, TokenMetadata } from "@/lib/tokens";
import { Milestone as MilestoneType } from "@/types";

interface MilestoneListProps {
  milestones: MilestoneType[];
  grantId: string;
  grantToken?: string;
}

interface _MilestoneDisplay extends MilestoneType {
  statusLabel: string;
  tokenSymbol: string;
  amountFormatted: string;
}

const STATUS_CLASSES: Record<string, string> = {
  Paid: "border border-success/40 text-success",
  Approved: "border border-accent-secondary/40 text-accent-secondary",
  Submitted: "border border-warning/40 text-warning",
  Overdue: "border border-danger/40 text-danger",
  "Due Soon": "border border-warning/40 text-warning",
  Pending: "border border-text-muted/40 text-text-muted",
};

export function MilestoneList({ milestones, grantId, grantToken }: MilestoneListProps) {
  const [tokenMetadataMap, setTokenMetadataMap] = useState<Map<string, TokenMetadata>>(new Map());
  const [isLoading, setIsLoading] = useState(true);

  const uniqueTokens = Array.from(
    new Set(
      milestones
        .map((m) => m.token || grantToken)
        .filter((t): t is string => !!t)
    )
  );

  useEffect(() => {
    async function fetchMetadata() {
      setIsLoading(true);
      const metadataMap = new Map<string, TokenMetadata>();
      for (const token of uniqueTokens) {
        const metadata = await getTokenMetadata(token);
        metadataMap.set(token, metadata);
      }
      setTokenMetadataMap(metadataMap);
      setIsLoading(false);
    }
    fetchMetadata();
  }, [uniqueTokens]);

  const getMilestoneStatus = (milestone: MilestoneType): string => {
    if (milestone.paid) return "Paid";
    if (milestone.approved) return "Approved";
    if (milestone.submitted) return "Submitted";
    if (milestone.overdue) return "Overdue";
    if ((milestone.daysUntilDeadline ?? Infinity) <= 7) return "Due Soon";
    return "Pending";
  };

  const formatMilestoneAmount = (milestone: MilestoneType): string => {
    const token = milestone.token || grantToken;
    if (!token || milestone.amount === undefined || milestone.amount === null) return "";
    const metadata = tokenMetadataMap.get(token);
    const decimals = metadata?.decimals ?? 7;
    const symbol = metadata?.symbol ?? "UNKNOWN";
    return formatTokenAmount(milestone.amount, decimals, { symbol, showSymbol: true });
  };

  if (isLoading) {
    return (
      <div className="space-y-4">
        {[1, 2, 3].map((i) => (
          <div key={i} className="border border-border-color rounded-none p-4 bg-bg-secondary">
            <div className="shimmer rounded-none h-4 w-3/4 mb-2" />
            <div className="shimmer rounded-none h-3 w-1/2" />
          </div>
        ))}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {milestones.map((milestone) => {
        const statusLabel = getMilestoneStatus(milestone);
        const statusClasses = STATUS_CLASSES[statusLabel] ?? STATUS_CLASSES.Pending;
        const amountFormatted = formatMilestoneAmount(milestone);

        return (
          <Link
            key={milestone.idx}
            href={`/grants/${grantId}/milestones/${milestone.idx}`}
            className="block border border-border-color rounded-none p-4 hover:border-accent-secondary/50 transition-colors bg-surface"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1">
                <div className="flex items-center gap-3 mb-2">
                  <span className="text-sm font-mono text-text-muted">#{milestone.idx}</span>
                  <h4 className="font-orbitron font-semibold text-lg">{milestone.title}</h4>
                </div>
                <p className="text-sm text-text-muted mb-3">{milestone.description}</p>
                <div className="flex items-center gap-4 text-sm">
                  <span
                    className={`inline-flex items-center px-2 py-0.5 rounded-none font-mono uppercase tracking-widest text-xs ${statusClasses}`}
                  >
                    {statusLabel}
                  </span>
                  {amountFormatted && (
                    <span className="font-medium">Payout: {amountFormatted}</span>
                  )}
                </div>
              </div>
            </div>
          </Link>
        );
      })}
    </div>
  );
}
