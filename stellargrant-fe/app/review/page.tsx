/**
 * Reviewer Dashboard
 *
 * Allows reviewers to view pending milestone votes and optionally vote
 * on multiple milestones in a single batched transaction (Batch Mode).
 *
 * Batch mode features:
 *  - "Batch Mode" toggle in the Pending tab
 *  - Per-card approve/reject mini buttons
 *  - Sticky bottom bar: "N milestones selected — X approvals, Y rejections"
 *  - ConfirmationDialog before submitting
 *  - Capped at 20 milestones (Stellar per-tx operation limit)
 *  - Success toast after batch execution
 */

"use client";

import { useState, useEffect, useContext, useCallback } from "react";
import { WalletGuard } from "@/components/wallet/WalletGuard";
import { useWalletStore } from "@/lib/store/walletStore";
import { useReputation } from "@/hooks/useReputation";
import { api } from "@/lib/api";
import { ConfirmationDialog } from "@/components/ui/ConfirmationDialog";
import { StellarGrantsContext } from "@/components/StellarGrantsProvider";
import Link from "next/link";

// ── Constants ─────────────────────────────────────────────────────────────────

const BATCH_MAX = 20;

// ── Types ─────────────────────────────────────────────────────────────────────

type FilterTab = "all" | "pending" | "voted";
type BatchVote = "approve" | "reject";

interface PendingMilestone {
  grantId: string;
  grantTitle: string;
  milestoneIdx: number;
  milestoneTitle: string;
  proofHash: string;
  submittedAt: string;
  votes: { reviewer: string; vote: "approve" | "reject" | null }[];
}

interface BatchSelection {
  /** milestoneKey → intended vote */
  [key: string]: BatchVote;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function milestoneKey(m: PendingMilestone): string {
  return `${m.grantId}-${m.milestoneIdx}`;
}

function relativeTime(dateStr: string): string {
  const diff = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)} minutes ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} hours ago`;
  if (diff < 2592000) return `${Math.floor(diff / 86400)} days ago`;
  return `${Math.floor(diff / 2592000)} months ago`;
}

// ── Sub-components ────────────────────────────────────────────────────────────

function Toast({
  message,
  onClose,
}: {
  message: string;
  onClose: () => void;
}) {
  useEffect(() => {
    const t = setTimeout(onClose, 4000);
    return () => clearTimeout(t);
  }, [onClose]);

  return (
    <div
      role="status"
      aria-live="polite"
      className="fixed bottom-6 right-6 z-50 flex items-center gap-3 bg-surface border border-success/40 px-5 py-3 font-mono text-sm text-success shadow-lg"
    >
      <span className="w-2 h-2 rounded-full bg-success animate-pulse flex-shrink-0" />
      {message}
      <button
        type="button"
        onClick={onClose}
        className="ml-2 text-text-muted hover:text-text-primary transition-colors"
        aria-label="Dismiss"
      >
        ✕
      </button>
    </div>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────

export default function ReviewerDashboard() {
  const address = useWalletStore((s) => s.address);
  const ctx = useContext(StellarGrantsContext);

  const [milestones, setMilestones] = useState<PendingMilestone[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [filter, setFilter] = useState<FilterTab>("all");
  const { score } = useReputation(address);

  // Batch state
  const [batchMode, setBatchMode] = useState(false);
  const [batchSelection, setBatchSelection] = useState<BatchSelection>({});
  const [isConfirmOpen, setIsConfirmOpen] = useState(false);
  const [isBatchExecuting, setIsBatchExecuting] = useState(false);
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  // ── Data fetch ──────────────────────────────────────────────────────────────

  const fetchMilestones = useCallback(async () => {
    if (!address) return;
    setIsLoading(true);
    try {
      const res = await api.get(`/reviewer/${address}/pending`);
      setMilestones(res.data.milestones ?? []);
    } catch {
      setMilestones([]);
    } finally {
      setIsLoading(false);
    }
  }, [address]);

  useEffect(() => {
    fetchMilestones();
  }, [fetchMilestones]);

  // ── Derived ─────────────────────────────────────────────────────────────────

  const filtered = milestones.filter((m) => {
    if (filter === "all") return true;
    const hasVoted = m.votes.some(
      (v) => v.reviewer === address && v.vote !== null
    );
    return filter === "pending" ? !hasVoted : hasVoted;
  });

  const pendingMilestones = milestones.filter(
    (m) => !m.votes.some((v) => v.reviewer === address && v.vote !== null)
  );
  const pendingCount = pendingMilestones.length;

  const selectedKeys = Object.keys(batchSelection);
  const approveCount = selectedKeys.filter(
    (k) => batchSelection[k] === "approve"
  ).length;
  const rejectCount = selectedKeys.filter(
    (k) => batchSelection[k] === "reject"
  ).length;

  // ── Batch helpers ───────────────────────────────────────────────────────────

  function toggleBatchMode() {
    setBatchMode((prev) => {
      if (prev) setBatchSelection({});
      return !prev;
    });
  }

  function setVote(m: PendingMilestone, vote: BatchVote) {
    const key = milestoneKey(m);
    setBatchSelection((prev) => {
      // Toggle off if same vote clicked again
      if (prev[key] === vote) {
        const next = { ...prev };
        delete next[key];
        return next;
      }
      if (Object.keys(prev).length >= BATCH_MAX && !prev[key]) {
        // Already at cap — ignore
        return prev;
      }
      return { ...prev, [key]: vote };
    });
  }

  function clearBatch() {
    setBatchSelection({});
  }

  // ── Batch execution ─────────────────────────────────────────────────────────

  async function executeBatch() {
    if (!ctx || selectedKeys.length === 0) return;
    setIsBatchExecuting(true);
    try {
      const batch = ctx.batch;
      batch.clear();

      for (const key of selectedKeys) {
        const vote = batchSelection[key];
        const [grantId, idxStr] = key.split("-");
        batch.add(vote === "approve" ? "milestoneApprove" : "milestoneReject", {
          grant_id: grantId,
          milestone_idx: Number(idxStr),
        });
      }

      const result = await batch.execute(async (method, args) => {
        // In production this calls contractClient[method](args) via Freighter.
        // Stub returns the method name as a pseudo-txHash for now.
        console.info(`[batch] executing ${method}`, args);
        return method;
      });

      const succeeded = result.operations.filter(
        (o) => o.status === "success"
      ).length;

      setToastMessage(
        result.allSucceeded
          ? `${succeeded} milestone${succeeded !== 1 ? "s" : ""} voted in one transaction`
          : `${succeeded} of ${selectedKeys.length} milestones submitted (some failed)`
      );

      clearBatch();
      setBatchMode(false);
      await fetchMilestones();
    } catch (err) {
      console.error("[batch] execution error", err);
    } finally {
      setIsBatchExecuting(false);
      setIsConfirmOpen(false);
    }
  }

  // ── Render ──────────────────────────────────────────────────────────────────

  return (
    <div className="min-h-screen bg-bg-primary text-text-primary">
      <div className="container mx-auto px-4 py-8">
        <WalletGuard>
          <div className="space-y-8">
            {/* Header */}
            <div className="flex items-center justify-between flex-wrap gap-4">
              <h1 className="font-orbitron text-2xl font-bold uppercase tracking-wider">
                Reviewer Dashboard
              </h1>
              {score !== null && (
                <div className="bg-surface border border-border-color px-4 py-2">
                  <span className="font-mono text-xs text-text-muted">
                    Reputation Score:{" "}
                    <span className="text-accent-primary font-bold">{score}</span>
                  </span>
                </div>
              )}
            </div>

            {/* Filter tabs */}
            <div className="flex gap-2 flex-wrap">
              {(
                [
                  ["all", "All"],
                  ["pending", "Pending My Vote"],
                  ["voted", "Already Voted"],
                ] as [FilterTab, string][]
              ).map(([key, label]) => (
                <button
                  key={key}
                  onClick={() => setFilter(key)}
                  className={`px-4 py-2 font-orbitron text-xs font-bold uppercase tracking-wider transition-all duration-300 ${
                    filter === key
                      ? "bg-accent-primary text-bg-primary"
                      : "bg-surface border border-border-color text-text-primary hover:bg-bg-secondary"
                  }`}
                >
                  {label}
                  {key === "pending" && pendingCount > 0 && (
                    <span className="ml-2 bg-danger text-bg-primary px-1.5 py-0.5 text-[10px] rounded-full">
                      {pendingCount}
                    </span>
                  )}
                </button>
              ))}
            </div>

            {/* Batch Mode toggle — only visible in Pending tab */}
            {filter === "pending" && pendingCount > 0 && (
              <div className="flex items-center gap-3">
                <button
                  type="button"
                  id="batch-mode-toggle"
                  onClick={toggleBatchMode}
                  className={`inline-flex items-center gap-2 px-4 py-2 font-orbitron text-xs font-bold uppercase tracking-wider border transition-all duration-200 ${
                    batchMode
                      ? "bg-accent-primary text-bg-primary border-accent-primary"
                      : "bg-transparent border-border-color text-text-muted hover:border-accent-primary hover:text-text-primary"
                  }`}
                  aria-pressed={batchMode}
                >
                  <span className="text-sm">{batchMode ? "☑" : "☐"}</span>
                  Batch Mode
                </button>
                {batchMode && (
                  <p className="font-mono text-xs text-text-muted">
                    Select up to {BATCH_MAX} milestones to vote in one
                    transaction.
                  </p>
                )}
              </div>
            )}

            {/* Milestone list */}
            {isLoading ? (
              <div className="text-center py-12">
                <div className="inline-block w-6 h-6 border-2 border-accent-primary border-t-transparent rounded-full animate-spin" />
              </div>
            ) : filtered.length === 0 ? (
              <div className="bg-surface border border-border-color p-12 text-center">
                <p className="font-mono text-text-muted">
                  {filter === "all"
                    ? "You're not a reviewer on any active grants"
                    : filter === "pending"
                    ? "You're all caught up — no pending votes"
                    : "You haven't voted on any milestones yet"}
                </p>
              </div>
            ) : (
              <div className="space-y-4">
                {filtered.map((m) => {
                  const key = milestoneKey(m);
                  const approvalCount = m.votes.filter(
                    (v) => v.vote === "approve"
                  ).length;
                  const hasVoted = m.votes.some(
                    (v) => v.reviewer === address && v.vote !== null
                  );
                  const myVote = m.votes.find((v) => v.reviewer === address);
                  const batchVote = batchSelection[key];
                  const isAtCap =
                    selectedKeys.length >= BATCH_MAX && !batchVote;

                  return (
                    <div
                      key={key}
                      className={`relative bg-surface border border-border-color p-6 space-y-3 transition-all duration-150 ${
                        batchVote
                          ? "border-accent-primary/50"
                          : ""
                      }`}
                    >
                      <Link
                        href={`/grants/${m.grantId}`}
                        className="font-orbitron text-sm font-bold uppercase tracking-wider text-accent-secondary hover:underline"
                      >
                        {m.grantTitle || `Grant #${m.grantId}`}
                      </Link>
                      <p className="font-mono text-sm text-text-primary">
                        Milestone: {m.milestoneTitle || `#${m.milestoneIdx}`}
                      </p>
                      <div className="flex items-center gap-4">
                        <span className="font-mono text-xs text-text-muted">
                          Submitted {relativeTime(m.submittedAt)}
                        </span>
                        {!hasVoted && (
                          <span className="flex items-center gap-1 font-mono text-xs text-warning">
                            <span className="inline-block w-1.5 h-1.5 bg-warning rounded-full animate-pulse" />
                            NEEDS VOTE
                          </span>
                        )}
                      </div>
                      <p className="font-mono text-xs text-text-muted">
                        Vote tally: {approvalCount} of 5 approved
                      </p>
                      {m.proofHash && (
                        <p className="font-mono text-[10px] text-text-muted">
                          Proof: {m.proofHash.slice(0, 20)}...
                        </p>
                      )}

                      {/* Actions: standard vs batch mode */}
                      {batchMode && !hasVoted ? (
                        <div className="flex items-center gap-2 pt-1">
                          <button
                            type="button"
                            disabled={isAtCap && batchVote !== "approve"}
                            onClick={() => setVote(m, "approve")}
                            aria-pressed={batchVote === "approve"}
                            className={`px-3 py-1.5 font-orbitron text-[10px] font-bold uppercase tracking-wider border transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed ${
                              batchVote === "approve"
                                ? "bg-success text-bg-primary border-success"
                                : "bg-transparent border-success/40 text-success hover:bg-success/10"
                            }`}
                          >
                            ✓ Approve
                          </button>
                          <button
                            type="button"
                            disabled={isAtCap && batchVote !== "reject"}
                            onClick={() => setVote(m, "reject")}
                            aria-pressed={batchVote === "reject"}
                            className={`px-3 py-1.5 font-orbitron text-[10px] font-bold uppercase tracking-wider border transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed ${
                              batchVote === "reject"
                                ? "bg-danger text-bg-primary border-danger"
                                : "bg-transparent border-danger/40 text-danger hover:bg-danger/10"
                            }`}
                          >
                            ✗ Reject
                          </button>
                        </div>
                      ) : hasVoted && myVote ? (
                        <span
                          className={`inline-block font-mono text-xs px-3 py-1 ${
                            myVote.vote === "approve"
                              ? "bg-success/10 text-success"
                              : "bg-danger/10 text-danger"
                          }`}
                        >
                          You voted:{" "}
                          {myVote.vote === "approve"
                            ? "Approved ✓"
                            : "Rejected ✗"}
                        </span>
                      ) : !batchMode ? (
                        <Link
                          href={`/grants/${m.grantId}/milestones/${m.milestoneIdx}`}
                          className="inline-block px-4 py-2 bg-accent-primary text-bg-primary font-orbitron text-xs font-bold uppercase tracking-wider transition-all duration-300 hover:opacity-90"
                        >
                          View Proof &amp; Vote →
                        </Link>
                      ) : null}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </WalletGuard>
      </div>

      {/* ── Sticky batch bottom bar ──────────────────────────────────────── */}
      {batchMode && selectedKeys.length > 0 && (
        <div
          role="region"
          aria-label="Batch voting controls"
          className="fixed bottom-0 inset-x-0 z-40 bg-surface border-t border-border-color shadow-2xl"
        >
          {selectedKeys.length >= BATCH_MAX && (
            <div className="bg-warning/10 border-b border-warning/30 px-4 py-1 text-center">
              <p className="font-mono text-xs text-warning">
                Batch capped at {BATCH_MAX} milestones (Stellar per-tx operation
                limit).
              </p>
            </div>
          )}
          <div className="container mx-auto px-4 py-4 flex flex-wrap items-center justify-between gap-4">
            <p className="font-mono text-sm text-text-primary">
              <span className="text-accent-primary font-bold">
                {selectedKeys.length}
              </span>{" "}
              milestone{selectedKeys.length !== 1 ? "s" : ""} selected —{" "}
              <span className="text-success">{approveCount} approval{approveCount !== 1 ? "s" : ""}</span>
              {", "}
              <span className="text-danger">{rejectCount} rejection{rejectCount !== 1 ? "s" : ""}</span>
            </p>
            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={() => setIsConfirmOpen(true)}
                disabled={isBatchExecuting}
                className="px-5 py-2 bg-success text-bg-primary font-orbitron text-xs font-bold uppercase tracking-wider hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
              >
                ✓ Submit Votes
              </button>
              <button
                type="button"
                onClick={clearBatch}
                className="px-5 py-2 bg-transparent border border-border-color text-text-muted font-orbitron text-xs font-bold uppercase tracking-wider hover:border-accent-primary hover:text-text-primary transition-all"
              >
                Clear
              </button>
              <button
                type="button"
                onClick={toggleBatchMode}
                className="px-5 py-2 bg-transparent border border-danger/40 text-danger font-orbitron text-xs font-bold uppercase tracking-wider hover:bg-danger/10 transition-all"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Confirmation dialog ──────────────────────────────────────────── */}
      <ConfirmationDialog
        isOpen={isConfirmOpen}
        onCancel={() => setIsConfirmOpen(false)}
        onConfirm={executeBatch}
        isLoading={isBatchExecuting}
        title="Confirm Batch Vote"
        description={`You are about to submit votes for ${selectedKeys.length} milestone${selectedKeys.length !== 1 ? "s" : ""} (${approveCount} approve, ${rejectCount} reject) in a single transaction. This cannot be undone.`}
        confirmLabel="Submit"
        variant="default"
      />

      {/* ── Success toast ────────────────────────────────────────────────── */}
      {toastMessage && (
        <Toast message={toastMessage} onClose={() => setToastMessage(null)} />
      )}
    </div>
  );
}
