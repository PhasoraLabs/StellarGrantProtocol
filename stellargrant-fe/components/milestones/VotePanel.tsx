"use client";

/**
 * VotePanel Component
 *
 * Displays the reviewer vote tally for a milestone and, when the
 * connected wallet is an eligible reviewer, surfaces Approve / Reject
 * action buttons.
 *
 * Consumes useVoting for all state management — optimistic updates,
 * Freighter signing, and success/error toasts are handled there.
 */

import { useVoting } from "@/hooks/useVoting";
import { useWalletStore } from "@/lib/store/walletStore";
import { Badge } from "@/components/ui/Badge";
import type { MilestoneVote } from "@/types";

// ─── Types ───────────────────────────────────────────────────────────────────

export interface VotePanelProps {
  grantId: string;
  milestoneIdx: number;
  /** Full reviewer address list so we can mark non-voters as "pending" */
  reviewers: string[];
  /** Required approve count for the milestone to pass */
  quorum: number;
  /**
   * Fraction of approvals needed (e.g. 0.67).
   * Not used for gating the button — that's the contract's job — but
   * shown in the UI as a threshold hint.
   */
  threshold?: number;
}

// ─── Small helpers ────────────────────────────────────────────────────────────

function shortenAddress(addr: string, chars = 6): string {
  if (addr.length <= chars * 2 + 2) return addr;
  return `${addr.slice(0, chars)}…${addr.slice(-chars)}`;
}

function ReviewerRow({
  reviewer,
  vote,
}: {
  reviewer: string;
  vote: MilestoneVote["vote"];
}) {
  const icon =
    vote === "approve" ? (
      <Badge variant="success" size="sm">
        ✓ Approved
      </Badge>
    ) : vote === "reject" ? (
      <Badge variant="danger" size="sm">
        ✗ Rejected
      </Badge>
    ) : (
      <Badge variant="muted" size="sm">
        — Pending
      </Badge>
    );

  return (
    <div className="flex items-center justify-between py-1.5 border-b border-border-color/30 last:border-b-0">
      <span className="font-mono text-xs text-text-muted">
        {shortenAddress(reviewer)}
      </span>
      {icon}
    </div>
  );
}

// ─── Component ────────────────────────────────────────────────────────────────

export function VotePanel({
  grantId,
  milestoneIdx,
  reviewers,
  quorum,
  threshold = 0.67,
}: VotePanelProps) {
  const { address: walletAddress } = useWalletStore();
  const { hasVoted, currentVote, votes, voteCount, isSubmitting, vote, error } =
    useVoting({
      grantId,
      milestoneIdx,
    });

  const isReviewer = !!walletAddress && reviewers.includes(walletAddress);
  const approvalPct =
    voteCount.total > 0
      ? Math.round((voteCount.approved / voteCount.total) * 100)
      : 0;
  const quorumReached = voteCount.approved >= quorum;

  // Build a lookup: reviewer address → their cast vote (null = not yet voted).
  // Seed every known reviewer as null first, then overwrite with actual votes.
  const voteByReviewer = new Map<string, MilestoneVote["vote"]>();
  reviewers.forEach((r) => voteByReviewer.set(r, null));
  votes.forEach((v) => {
    if (voteByReviewer.has(v.reviewer)) {
      voteByReviewer.set(v.reviewer, v.vote);
    }
  });

  return (
    <div className="space-y-5">
      {/* ── Tally header ───────────────────────────────────────────────── */}
      <div className="flex items-center justify-between">
        <h3 className="font-orbitron text-sm uppercase tracking-widest">
          Reviewer Votes
        </h3>
        <Badge variant={quorumReached ? "success" : "muted"}>
          {voteCount.approved} / {quorum} approved
        </Badge>
      </div>

      {/* ── Quorum progress bar ─────────────────────────────────────────── */}
      <div>
        <div className="h-1.5 w-full bg-surface rounded-none overflow-hidden">
          <div
            className="h-full bg-success transition-all duration-500"
            style={{ width: `${Math.min(approvalPct, 100)}%` }}
          />
        </div>
        <p className="mt-1 text-right font-mono text-[10px] text-text-muted">
          {approvalPct}% approved · {Math.round(threshold * 100)}% threshold
        </p>
      </div>

      {/* ── Reviewer list ───────────────────────────────────────────────── */}
      {reviewers.length > 0 ? (
        <div className="rounded-none border border-border-color/40 px-3 py-1">
          {reviewers.map((r) => (
            <ReviewerRow
              key={r}
              reviewer={r}
              vote={voteByReviewer.get(r) ?? null}
            />
          ))}
        </div>
      ) : (
        <p className="font-mono text-xs text-text-muted">
          No reviewers assigned.
        </p>
      )}

      {/* ── Action buttons (reviewer-only) ──────────────────────────────── */}
      {isReviewer && (
        <div className="space-y-3 pt-1">
          {hasVoted ? (
            <div className="rounded-none border border-border-color/40 p-3">
              <p className="font-mono text-xs text-text-muted">
                Already voted —{" "}
                <span className={currentVote ? "text-success" : "text-danger"}>
                  {currentVote ? "Approved ✓" : "Rejected ✗"}
                </span>
              </p>
            </div>
          ) : (
            <div className="flex gap-3">
              <button
                onClick={() => void vote(true)}
                disabled={isSubmitting}
                className="flex-1 rounded-none border border-success/40 bg-success/10 py-2 font-mono text-xs uppercase tracking-widest text-success transition-colors hover:bg-success/20 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSubmitting ? "Submitting…" : "✓ Approve"}
              </button>
              <button
                onClick={() => void vote(false)}
                disabled={isSubmitting}
                className="flex-1 rounded-none border border-danger/40 bg-danger/10 py-2 font-mono text-xs uppercase tracking-widest text-danger transition-colors hover:bg-danger/20 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSubmitting ? "Submitting…" : "✗ Reject"}
              </button>
            </div>
          )}
        </div>
      )}

      {/* ── Error message ───────────────────────────────────────────────── */}
      {error && (
        <p className="rounded-none border border-danger/40 bg-danger/10 px-3 py-2 font-mono text-xs text-danger">
          {error}
        </p>
      )}
    </div>
  );
}
