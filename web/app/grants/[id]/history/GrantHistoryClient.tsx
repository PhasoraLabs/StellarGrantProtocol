"use client";

import Link from "next/link";
import type { GrantHistoryRecord, GrantOperationType } from "@/lib/stellar/history";
import { useGrantHistory } from "@/hooks/useGrantHistory";
import { useRelativeTime } from "@/hooks/useRelativeTime";
import { WalletAddress } from "@/components/wallet/WalletAddress";
import { stellarExplorerTx } from "@/lib/constants";

const LABELS: Record<GrantOperationType, string> = {
  grant_create: "Grant Created",
  grant_fund: "Funding Deposit",
  grant_cancel: "Grant Cancelled",
  milestone_submit: "Proof Submitted",
  milestone_approve: "Milestone Approved",
  milestone_reject: "Milestone Rejected",
  milestone_payout: "Payout Released",
  grant_withdraw: "Funds Withdrawn",
  unknown_contract_call: "Contract Call",
};

function operationDotClass(type: GrantOperationType): string {
  switch (type) {
    case "grant_fund":
      return "bg-success";
    case "milestone_approve":
    case "milestone_payout":
      return "bg-accent-primary";
    case "milestone_reject":
    case "grant_cancel":
      return "bg-danger";
    default:
      return "bg-accent-secondary";
  }
}

function formatHistoryDetail(record: GrantHistoryRecord): string {
  const memo = record.memo?.trim();
  const grantMemo = record.grantId ? `grant:${record.grantId}` : undefined;
  const extra =
    memo && memo.toLowerCase() !== grantMemo?.toLowerCase() ? memo : undefined;

  if (extra) {
    if (record.operationType === "milestone_approve") {
      return extra.startsWith("✓") ? extra : `✓ ${extra}`;
    }
    if (record.operationType === "grant_fund" && !/^[+-]/.test(extra)) {
      return `+${extra}`;
    }
    return extra;
  }

  switch (record.operationType) {
    case "grant_fund":
      return "Deposit";
    case "milestone_submit":
      return "Proof submitted";
    case "milestone_approve":
      return "Approved";
    case "milestone_reject":
      return "Rejected";
    case "milestone_payout":
      return "Payout";
    case "grant_create":
      return "Grant created";
    case "grant_cancel":
      return "Cancelled";
    case "grant_withdraw":
      return "Withdrawal";
    default:
      return record.successful ? "On-chain activity" : "Failed transaction";
  }
}

function HistoryRow({ record }: { record: GrantHistoryRecord }) {
  const relativeTime = useRelativeTime(record.createdAt);
  const txSuffix = record.txHash.slice(-8);

  return (
    <li className="group grid grid-cols-[auto_1fr_auto_auto_auto] items-center gap-3 border-b border-border-color/40 px-4 py-3 font-mono text-xs transition-colors hover:bg-surface/50 last:border-b-0 sm:gap-4">
      <span
        className={`h-2 w-2 shrink-0 rounded-full ${operationDotClass(record.operationType)}`}
        aria-hidden="true"
      />
      <span className="min-w-0 truncate text-text-primary">
        {LABELS[record.operationType]}
      </span>
      <span className="hidden min-w-0 sm:block">
        <WalletAddress address={record.sourceAccount} showCopyIcon />
      </span>
      <span className="min-w-0 truncate text-right text-text-muted sm:text-text-primary">
        {formatHistoryDetail(record)}
      </span>
      <div className="flex shrink-0 flex-col items-end gap-0.5 text-right">
        <span className="text-text-muted whitespace-nowrap">{relativeTime}</span>
        <a
          href={stellarExplorerTx(record.txHash)}
          target="_blank"
          rel="noopener noreferrer"
          className="text-accent-secondary hover:underline"
          title={record.txHash}
        >
          …{txSuffix}
        </a>
      </div>
      <span className="col-span-full sm:hidden">
        <WalletAddress address={record.sourceAccount} showCopyIcon />
      </span>
    </li>
  );
}

export function GrantHistoryClient({ grantId }: { grantId: string }) {
  const { records, isLoading, error, hasMore, loadMore, refetch } =
    useGrantHistory(grantId);

  return (
    <div className="container mx-auto max-w-4xl px-4 py-8">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-4">
        <h1 className="font-orbitron text-xl uppercase tracking-wider text-text-primary sm:text-2xl">
          Grant Transaction History
        </h1>
        <div className="flex items-center gap-4">
          <Link
            href={`/grants/${grantId}`}
            className="font-mono text-xs text-accent-secondary hover:underline"
          >
            ← Back to Grant
          </Link>
          <button
            type="button"
            onClick={() => void refetch()}
            disabled={isLoading}
            className="font-mono text-xs uppercase tracking-wider text-accent-primary hover:underline disabled:opacity-50"
          >
            Refresh
          </button>
        </div>
      </div>

      {error && (
        <div className="mb-4 rounded-none border border-danger/40 bg-danger/10 p-4 font-mono text-xs text-danger">
          {error.message}
        </div>
      )}

      <div className="border border-border-color bg-surface ring-1 ring-border-color">
        {isLoading && records.length === 0 ? (
          <div className="space-y-0">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="shimmer h-14 border-b border-border-color/40" />
            ))}
          </div>
        ) : records.length === 0 ? (
          <p className="p-8 text-center font-mono text-sm text-text-muted">
            No on-chain activity recorded yet for this grant.
          </p>
        ) : (
          <ul>
            {records.map((record) => (
              <HistoryRow key={record.txHash} record={record} />
            ))}
          </ul>
        )}
      </div>

      {hasMore && (
        <div className="mt-6 text-center">
          <button
            type="button"
            onClick={() => void loadMore()}
            disabled={isLoading}
            className="font-orbitron text-sm font-bold uppercase tracking-wider border border-accent-primary px-6 py-2 text-accent-primary transition-colors hover:bg-accent-primary hover:text-bg-primary disabled:opacity-50"
          >
            Load More
          </button>
        </div>
      )}
    </div>
  );
}
