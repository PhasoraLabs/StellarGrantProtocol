"use client";

import React, { useMemo } from "react";
import type { GrantHistoryRecord } from "@/lib/stellar/history";

interface TimelineGroup {
  date: string;
  records: GrantHistoryRecord[];
}

export interface GrantHistoryTimelineProps {
  records: GrantHistoryRecord[];
  isLoading?: boolean;
  error?: Error | null;
}

function formatDate(isoString: string): string {
  const date = new Date(isoString);
  return date.toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

function formatTime(isoString: string): string {
  const date = new Date(isoString);
  return date.toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: true,
  });
}

function getOperationIcon(operationType: string): string {
  const iconMap: Record<string, string> = {
    grant_create: "✨",
    grant_fund: "💰",
    grant_cancel: "❌",
    milestone_submit: "📝",
    milestone_approve: "✅",
    milestone_reject: "👎",
    milestone_payout: "💸",
    grant_withdraw: "🏦",
    unknown_contract_call: "⚙️",
  };
  return iconMap[operationType] || "📌";
}

function getOperationLabel(operationType: string): string {
  const labelMap: Record<string, string> = {
    grant_create: "Grant Created",
    grant_fund: "Grant Funded",
    grant_cancel: "Grant Cancelled",
    milestone_submit: "Milestone Submitted",
    milestone_approve: "Milestone Approved",
    milestone_reject: "Milestone Rejected",
    milestone_payout: "Milestone Payout",
    grant_withdraw: "Funds Withdrawn",
    unknown_contract_call: "Contract Call",
  };
  return labelMap[operationType] || "Event";
}

export function GrantHistoryTimeline({
  records,
  isLoading = false,
  error = null,
}: GrantHistoryTimelineProps) {
  const groupedRecords = useMemo<TimelineGroup[]>(() => {
    if (!records.length) return [];

    const groups: Record<string, GrantHistoryRecord[]> = {};
    records.forEach((record) => {
      const date = formatDate(record.createdAt);
      if (!groups[date]) {
        groups[date] = [];
      }
      groups[date].push(record);
    });

    return Object.entries(groups)
      .sort(([dateA], [dateB]) => new Date(dateB).getTime() - new Date(dateA).getTime())
      .map(([date, recordsList]) => ({
        date,
        records: recordsList.sort(
          (a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime(),
        ),
      }));
  }, [records]);

  if (isLoading) {
    return (
      <div className="flex justify-center items-center py-8">
        <div className="text-text-muted font-mono text-sm">Loading history...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="border border-danger bg-danger/10 rounded p-4">
        <p className="text-danger font-mono text-sm">Error loading history: {error.message}</p>
      </div>
    );
  }

  if (groupedRecords.length === 0) {
    return (
      <div className="text-center py-8">
        <p className="text-text-muted font-mono text-sm">No history records found</p>
      </div>
    );
  }

  return (
    <div className="relative">
      {groupedRecords.map((group, groupIdx) => (
        <div key={groupIdx} className="mb-8">
          <h3 className="font-orbitron text-sm font-bold text-accent-primary mb-4">
            {group.date}
          </h3>

          <div className="relative pl-8">
            {group.records.map((record, recordIdx) => (
              <div key={record.txHash} className="mb-4 relative">
                <div className="absolute left-0 top-2 w-4 h-4 bg-accent-primary rounded-full transform -translate-x-6" />

                {recordIdx < group.records.length - 1 && (
                  <div className="absolute left-0 top-6 w-0.5 h-12 bg-border-color transform -translate-x-5.5" />
                )}

                <div
                  className={`border border-border-color rounded p-3 ${
                    record.successful
                      ? "bg-bg-surface hover:bg-opacity-70"
                      : "bg-danger/5 border-danger/30"
                  } transition-colors`}
                >
                  <div className="flex items-start gap-3">
                    <span className="text-lg flex-shrink-0">
                      {getOperationIcon(record.operationType)}
                    </span>
                    <div className="flex-1">
                      <div className="flex items-center justify-between">
                        <p className="font-mono text-sm font-bold text-text-primary">
                          {getOperationLabel(record.operationType)}
                        </p>
                        <span className="text-xs text-text-muted">
                          {formatTime(record.createdAt)}
                        </span>
                      </div>
                      {!record.successful && (
                        <p className="text-xs text-danger mt-1">Transaction failed</p>
                      )}
                      <p className="text-xs text-text-muted mt-1 font-mono">
                        {record.txHash.slice(0, 16)}...
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
