"use client";

import { useRef, useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { useVoting } from "@/hooks/useVoting";
import { useWalletStore } from "@/lib/store/walletStore";
import { Badge } from "@/components/ui/Badge";
import { ConfirmationDialog } from "@/components/ui/ConfirmationDialog";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import type { MilestoneVote } from "@/types";
import type { ConnectionStatus } from "@/hooks/useContractEvents";

export interface VotePanelProps {
  grantId: string;
  milestoneIdx: number;
  reviewers: string[];
  quorum: number;
  threshold?: number;
  connectionStatus?: ConnectionStatus;
}

function shortenAddress(addr: string, chars = 6): string {
  if (addr.length <= chars * 2 + 2) return addr;
  return `${addr.slice(0, chars)}…${addr.slice(-chars)}`;
}

function ReviewerRow({
  reviewer,
  vote,
  isPending,
}: {
  reviewer: string;
  vote: MilestoneVote["vote"];
  isPending?: boolean;
}) {
  const icon = isPending ? (
    <span className="inline-flex items-center gap-1 font-mono text-[9px] uppercase tracking-widest text-warning">
      <span className="inline-block w-1.5 h-1.5 bg-warning rounded-full animate-pulse" />
      Pending
    </span>
  ) : vote === "approve" ? (
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

export function VotePanel({
  grantId,
  milestoneIdx,
  reviewers,
  quorum,
  threshold = 0.67,
  connectionStatus,
}: VotePanelProps) {
  const { address: walletAddress } = useWalletStore();
  const { hasVoted, currentVote, votes, voteCount, isSubmitting, vote, error } =
    useVoting({ grantId, milestoneIdx });

  const [confirmVoteType, setConfirmVoteType] = useState<"approve" | "reject" | null>(null);

  const isReviewer = !!walletAddress && reviewers.includes(walletAddress);
  const approvalPct =
    voteCount.total > 0
      ? Math.round((voteCount.approved / voteCount.total) * 100)
      : 0;
  const quorumReached = voteCount.approved >= quorum;

  const prevQuorumRef = useRef(quorumReached);
  const [showQuorumBanner, setShowQuorumBanner] = useState(false);
  const [prevApprovalCount, setPrevApprovalCount] = useState(
    voteCount.approved,
  );

  useEffect(() => {
    if (quorumReached && !prevQuorumRef.current) {
      setShowQuorumBanner(true);
      const timer = setTimeout(() => setShowQuorumBanner(false), 6000);
      window.dispatchEvent(
        new CustomEvent("stellar:toast", {
          detail: {
            type: "vote_recorded",
            title: "Quorum reached",
            message: "Milestone approved by quorum! Payout will be processed.",
          },
        }),
      );
      return () => clearTimeout(timer);
    }
    prevQuorumRef.current = quorumReached;
  }, [quorumReached]);

  useEffect(() => {
    if (voteCount.approved !== prevApprovalCount) {
      setPrevApprovalCount(voteCount.approved);
    }
  }, [voteCount.approved, prevApprovalCount]);

  const voteByReviewer = new Map<string, MilestoneVote["vote"]>();
  reviewers.forEach((r) => voteByReviewer.set(r, null));
  votes.forEach((v) => {
    if (voteByReviewer.has(v.reviewer)) {
      voteByReviewer.set(v.reviewer, v.vote);
    }
  });

  useKeyboardShortcuts([
    {
      key: "a",
      description: "Approve Milestone",
      condition: () => isReviewer && !hasVoted && !isSubmitting && confirmVoteType === null,
      action: (e) => {
        e?.preventDefault();
        setConfirmVoteType("approve");
      },
    },
    {
      key: "r",
      description: "Reject Milestone",
      condition: () => isReviewer && !hasVoted && !isSubmitting && confirmVoteType === null,
      action: (e) => {
        e?.preventDefault();
        setConfirmVoteType("reject");
      },
    },
  ]);

  const liveColor =
    connectionStatus === "connected"
      ? "text-success"
      : connectionStatus === "connecting"
        ? "text-warning"
        : "text-danger";

  return (
    <div className="space-y-5">
      <AnimatePresence>
        {showQuorumBanner && (
          <motion.div
            initial={{ y: -40, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: -40, opacity: 0 }}
            transition={{ type: "spring", stiffness: 300, damping: 25 }}
            className="bg-success/10 border border-success/40 px-4 py-2 text-center"
          >
            <span className="font-orbitron text-sm font-bold text-success">
              ✓ Quorum reached
            </span>
          </motion.div>
        )}
      </AnimatePresence>

      <div className="flex items-center justify-between">
        <h3 className="font-orbitron text-sm uppercase tracking-widest">
          Reviewer Votes
        </h3>
        <div className="flex items-center gap-3">
          {connectionStatus && (
            <span
              className={`inline-flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider ${liveColor}`}
            >
              <span
                className={`inline-block w-1.5 h-1.5 rounded-full ${liveColor === "text-success" ? "bg-success" : liveColor === "text-warning" ? "bg-warning" : "bg-danger"}`}
              />
              LIVE
            </span>
          )}
          <Badge variant={quorumReached ? "success" : "muted"}>
            {voteCount.approved} / {quorum} approved
          </Badge>
        </div>
      </div>

      <div aria-live="polite" className="sr-only">
        {`Vote updated: ${voteCount.approved} of ${quorum} approvals`}
      </div>

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

      {reviewers.length > 0 ? (
        <div className="rounded-none border border-border-color/40 px-3 py-1">
          {reviewers.map((r) => {
            const isWalletPending =
              isSubmitting && r === walletAddress && !voteByReviewer.get(r);
            return (
              <ReviewerRow
                key={r}
                reviewer={r}
                vote={voteByReviewer.get(r) ?? null}
                isPending={isWalletPending}
              />
            );
          })}
        </div>
      ) : (
        <p className="font-mono text-xs text-text-muted">
          No reviewers assigned.
        </p>
      )}

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
                onClick={() => setConfirmVoteType("approve")}
                disabled={isSubmitting}
                className="flex-1 rounded-none border border-success/40 bg-success/10 py-2 font-mono text-xs uppercase tracking-widest text-success transition-colors hover:bg-success/20 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSubmitting && confirmVoteType === "approve" ? "Submitting…" : "✓ Approve"}
              </button>
              <button
                onClick={() => setConfirmVoteType("reject")}
                disabled={isSubmitting}
                className="flex-1 rounded-none border border-danger/40 bg-danger/10 py-2 font-mono text-xs uppercase tracking-widest text-danger transition-colors hover:bg-danger/20 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSubmitting && confirmVoteType === "reject" ? "Submitting…" : "✗ Reject"}
              </button>
            </div>
          )}
        </div>
      )}

      {error && (
        <p className="rounded-none border border-danger/40 bg-danger/10 px-3 py-2 font-mono text-xs text-danger">
          {error}
        </p>
      )}

      <ConfirmationDialog
        isOpen={confirmVoteType !== null}
        title={confirmVoteType === "approve" ? "Approve Milestone" : "Reject Milestone"}
        description={
          confirmVoteType === "approve"
            ? "Are you sure you want to approve this milestone? This action cannot be undone."
            : "Are you sure you want to reject this milestone? This action cannot be undone."
        }
        confirmLabel={confirmVoteType === "approve" ? "Yes, Approve" : "Yes, Reject"}
        variant={confirmVoteType === "approve" ? "default" : "danger"}
        isLoading={isSubmitting}
        onConfirm={async () => {
          if (confirmVoteType === "approve") {
            await vote(true);
          } else if (confirmVoteType === "reject") {
            await vote(false);
          }
          setConfirmVoteType(null);
        }}
        onCancel={() => setConfirmVoteType(null)}
      />
    </div>
  );
}
