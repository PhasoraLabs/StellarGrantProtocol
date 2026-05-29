"use client";

/**
 * useVoting Hook
 *
 * Manages the full voting flow for a single milestone:
 *   1. Derives hasVoted / currentVote from the milestone's votes array.
 *   2. Submits an approve or reject vote via the ContractClient.
 *   3. Applies optimistic UI updates immediately, reverts on failure.
 *   4. Fires a success/failure toast via the `stellar:toast` CustomEvent
 *      consumed by NotificationToast.
 *
 * Usage:
 *   const { hasVoted, currentVote, votes, voteCount, isSubmitting, vote, error } =
 *     useVoting({ grantId: "42", milestoneIdx: 1 });
 */

import { useState, useEffect, useCallback, useRef } from "react";
import { useWalletStore } from "@/lib/store/walletStore";
import { useMilestone } from "./useMilestone";
import { contractClient } from "@/lib/stellar/contract";
import type { ToastEventDetail } from "@/components/ui/NotificationToast";
import type { MilestoneVote } from "@/types";

// ─── Public types ────────────────────────────────────────────────────────────

export interface UseVotingOptions {
  grantId: string;
  milestoneIdx: number;
}

export interface VoteCount {
  approved: number;
  rejected: number;
  total: number;
}

export interface UseVotingReturn {
  /** Whether the connected wallet has already cast a vote */
  hasVoted: boolean;
  /** The vote the connected wallet cast — true = approved, false = rejected, null = not voted */
  currentVote: boolean | null;
  /** Full per-reviewer vote list (for rendering reviewer rows in VotePanel) */
  votes: MilestoneVote[];
  /** Aggregated vote tally */
  voteCount: VoteCount;
  /** True while the Freighter signing + tx submission is in flight */
  isSubmitting: boolean;
  /** Submit a vote. Pass true to approve, false to reject. */
  vote: (approve: boolean) => Promise<void>;
  /** Last error message, if any */
  error: string | null;
}

// ─── Toast helpers ───────────────────────────────────────────────────────────

function emitToast(detail: ToastEventDetail) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(
    new CustomEvent<ToastEventDetail>("stellar:toast", { detail }),
  );
}

function buildExplorerUrl(network: string): string {
  return network === "mainnet"
    ? "https://stellar.expert/explorer/public"
    : "https://stellar.expert/explorer/testnet";
}

// ─── Vote tally derivation ───────────────────────────────────────────────────

function deriveVoteCount(votes: MilestoneVote[]): VoteCount {
  return votes.reduce<VoteCount>(
    (acc, v) => {
      if (v.vote === "approve") acc.approved++;
      else if (v.vote === "reject") acc.rejected++;
      acc.total++;
      return acc;
    },
    { approved: 0, rejected: 0, total: 0 },
  );
}

function deriveHasVoted(
  votes: MilestoneVote[],
  walletAddress: string | null,
): { hasVoted: boolean; currentVote: boolean | null } {
  if (!walletAddress) return { hasVoted: false, currentVote: null };
  const mine = votes.find((v) => v.reviewer === walletAddress);
  if (!mine || mine.vote === null)
    return { hasVoted: false, currentVote: null };
  return { hasVoted: true, currentVote: mine.vote === "approve" };
}

// ─── Stable-ref sync helper ──────────────────────────────────────────────────
//
// useMilestone is currently a stub that returns a fresh `[]` literal on every
// render. Comparing by reference would trigger the sync useEffect on every
// tick, flooding the component with state updates.  We compare by a JSON
// snapshot so that an empty array stays stable, and real data only syncs once
// the content actually changes.

function votesEqual(a: MilestoneVote[], b: MilestoneVote[]): boolean {
  if (a.length !== b.length) return false;
  return a.every((av, i) => {
    const bv = b[i];
    return av.reviewer === bv.reviewer && av.vote === bv.vote;
  });
}

// ─── Hook ────────────────────────────────────────────────────────────────────

export function useVoting({
  grantId,
  milestoneIdx,
}: UseVotingOptions): UseVotingReturn {
  const { address: walletAddress, network } = useWalletStore();

  // Underlying milestone data (votes list) from the server/contract
  const { votes: liveVotes } = useMilestone(grantId, milestoneIdx);

  // Local votes state — starts empty, syncs from live data, gets optimistic
  // updates applied without waiting for a round-trip.
  const [votes, setVotes] = useState<MilestoneVote[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Keep a ref to the last synced liveVotes so we can compare content, not
  // reference identity (the stub always returns a new [] literal).
  const lastLiveRef = useRef<MilestoneVote[]>([]);

  useEffect(() => {
    if (!votesEqual(liveVotes, lastLiveRef.current)) {
      lastLiveRef.current = liveVotes;
      setVotes(liveVotes);
    }
  }, [liveVotes]);

  // ── Derived state ──────────────────────────────────────────────────────
  const { hasVoted, currentVote } = deriveHasVoted(votes, walletAddress);
  const voteCount = deriveVoteCount(votes);

  // ── vote() ─────────────────────────────────────────────────────────────
  const vote = useCallback(
    async (approve: boolean) => {
      if (!walletAddress) {
        setError("Connect your wallet to vote.");
        return;
      }
      if (hasVoted) {
        setError("You have already voted on this milestone.");
        return;
      }

      setError(null);
      setIsSubmitting(true);

      // ── Optimistic update ────────────────────────────────────────────
      const optimisticVote: MilestoneVote = {
        reviewer: walletAddress,
        vote: approve ? "approve" : "reject",
        voted_at: BigInt(Math.floor(Date.now() / 1000)),
      };

      setVotes((prev: MilestoneVote[]) => {
        const without = prev.filter(
          (v: MilestoneVote) => v.reviewer !== walletAddress,
        );
        return [...without, optimisticVote];
      });

      // ── Contract call ────────────────────────────────────────────────
      try {
        await contractClient.voteOnMilestone(grantId, milestoneIdx, approve);

        emitToast({
          type: "vote_recorded",
          title: "Vote recorded",
          message: `Your ${approve ? "approval" : "rejection"} was submitted successfully.`,
          href: buildExplorerUrl(network),
        });
      } catch (err) {
        // ── Revert optimistic state on failure ───────────────────────
        setVotes((prev: MilestoneVote[]) =>
          prev.filter((v: MilestoneVote) => v.reviewer !== walletAddress),
        );

        const msg =
          err instanceof Error
            ? err.message
            : "Transaction failed. Please try again.";
        setError(msg);

        emitToast({
          type: "vote_error",
          title: "Vote failed",
          message: msg,
        });
      } finally {
        setIsSubmitting(false);
      }
    },
    [walletAddress, hasVoted, grantId, milestoneIdx, network],
  );

  return { hasVoted, currentVote, votes, voteCount, isSubmitting, vote, error };
}
