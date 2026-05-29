"use client";

import { useState, useEffect } from "react";
import { WalletGuard } from "@/components/wallet/WalletGuard";
import { useWalletStore } from "@/lib/store/walletStore";
import { useReputation } from "@/hooks/useReputation";
import { api } from "@/lib/api";
import Link from "next/link";

type FilterTab = "all" | "pending" | "voted";

interface PendingMilestone {
  grantId: string;
  grantTitle: string;
  milestoneIdx: number;
  milestoneTitle: string;
  proofHash: string;
  submittedAt: string;
  votes: { reviewer: string; vote: "approve" | "reject" | null }[];
}

function relativeTime(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diff = Math.floor((now - then) / 1000);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)} minutes ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} hours ago`;
  if (diff < 2592000) return `${Math.floor(diff / 86400)} days ago`;
  return `${Math.floor(diff / 2592000)} months ago`;
}

export default function ReviewerDashboard() {
  const address = useWalletStore((s) => s.address);
  const [milestones, setMilestones] = useState<PendingMilestone[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [filter, setFilter] = useState<FilterTab>("all");
  const { score } = useReputation(address);

  useEffect(() => {
    if (!address) return;

    const fetchMilestones = async () => {
      setIsLoading(true);
      try {
        const res = await api.get(`/reviewer/${address}/pending`);
        setMilestones(res.data.milestones ?? []);
      } catch {
        setMilestones([]);
      } finally {
        setIsLoading(false);
      }
    };

    fetchMilestones();
  }, [address]);

  const filtered = milestones.filter((m) => {
    if (filter === "all") return true;
    const hasVoted = m.votes.some(
      (v) => v.reviewer === address && v.vote !== null,
    );
    return filter === "pending" ? !hasVoted : hasVoted;
  });

  const pendingCount = milestones.filter(
    (m) => !m.votes.some((v) => v.reviewer === address && v.vote !== null),
  ).length;

  return (
    <div className="min-h-screen bg-bg-primary text-text-primary">
      <div className="container mx-auto px-4 py-8">
        <WalletGuard>
          <div className="space-y-8">
            <div className="flex items-center justify-between">
              <h1 className="font-orbitron text-2xl font-bold uppercase tracking-wider">
                Reviewer Dashboard
              </h1>
              {score !== null && (
                <div className="bg-surface border border-border-color rounded-[4px] px-4 py-2">
                  <span className="font-mono text-xs text-text-muted">
                    Reputation Score:{" "}
                    <span className="text-accent-primary font-bold">
                      {score}
                    </span>
                  </span>
                </div>
              )}
            </div>

            <div className="flex gap-2">
              {(
                [
                  ["all", "All"],
                  ["pending", `Pending My Vote`],
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

            {isLoading ? (
              <div className="text-center py-12">
                <div className="inline-block w-6 h-6 border-2 border-accent-primary border-t-transparent rounded-full animate-spin" />
              </div>
            ) : filtered.length === 0 ? (
              <div className="bg-surface border border-border-color rounded-[4px] p-12 text-center">
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
                  const approvalCount = m.votes.filter(
                    (v) => v.vote === "approve",
                  ).length;
                  const hasVoted = m.votes.some(
                    (v) => v.reviewer === address && v.vote !== null,
                  );
                  const myVote = m.votes.find((v) => v.reviewer === address);

                  return (
                    <div
                      key={`${m.grantId}-${m.milestoneIdx}`}
                      className="bg-surface border border-border-color rounded-[4px] p-6 space-y-3"
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
                      {hasVoted && myVote ? (
                        <span
                          className={`inline-block font-mono text-xs px-3 py-1 rounded-[4px] ${
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
                      ) : (
                        <Link
                          href={`/grants/${m.grantId}/milestones/${m.milestoneIdx}`}
                          className="inline-block px-4 py-2 bg-accent-primary text-bg-primary font-orbitron text-xs font-bold uppercase tracking-wider transition-all duration-300 hover:opacity-90"
                        >
                          View Proof & Vote →
                        </Link>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </WalletGuard>
      </div>
    </div>
  );
}
