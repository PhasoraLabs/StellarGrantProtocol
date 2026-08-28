/**
 * useMilestone Hook
 *
 * Fetches milestone state for a specific grant + milestone index.
 * Includes vote data and submission proof.
 */

"use client";

import { useState, useEffect, useCallback } from "react";
import type { Milestone, MilestoneVote, Grant } from "@/types";
import { API_URL } from "@/lib/constants";
import { useWalletStore } from "@/lib/store/walletStore";

interface UseMilestoneReturn {
  milestone: Milestone | null;
  votes: MilestoneVote[];
  isReviewer: boolean;
  isRecipient: boolean;
  hasVoted: boolean;
  currentVote: boolean | null;
  quorum: number;
  approvalCount: number;
  rejectionCount: number;
  isLoading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

const POLL_INTERVAL_MS = 15000; // 15 seconds

export function useMilestone(grantId: string, milestoneIdx: number): UseMilestoneReturn {
  const { address } = useWalletStore();
  const [milestone, setMilestone] = useState<Milestone | null>(null);
  const [votes, setVotes] = useState<MilestoneVote[]>([]);
  const [grant, setGrant] = useState<Grant | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetchMilestone = useCallback(async () => {
    if (!grantId) return;

    queueMicrotask(() => setIsLoading(true));
    queueMicrotask(() => setError(null));

    await Promise.resolve();
    try {
      // Fetch grant data from API
      const response = await fetch(`${API_URL}/grants/${grantId}`);
      
      if (!response.ok) {
        throw new Error(`Failed to fetch grant: ${response.statusText}`);
      }

      const grantData: Grant & { milestones?: Milestone[] } = await response.json();
      setGrant(grantData);

      // Extract milestone at the specified index
      if (grantData.milestones && grantData.milestones[milestoneIdx]) {
        const milestoneData = grantData.milestones[milestoneIdx];
        setMilestone(milestoneData);

        // Extract votes if available
        const milestoneWithVotes = milestoneData as Milestone & { votes?: MilestoneVote[] };
        if (milestoneWithVotes.votes) {
          setVotes(milestoneWithVotes.votes);
        }
      } else {
        throw new Error(`Milestone ${milestoneIdx} not found`);
      }
    } catch (err) {
      const errorObj = err instanceof Error ? err : new Error("Failed to fetch milestone");
      setError(errorObj);
      console.error("Error fetching milestone:", err);
    } finally {
      setIsLoading(false);
    }
  }, [grantId, milestoneIdx]);

  // Initial fetch
  useEffect(() => {
    queueMicrotask(() => {
      fetchMilestone();
    });
  }, [fetchMilestone]);

  // Polling every 15 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      fetchMilestone();
    }, POLL_INTERVAL_MS);

    return () => clearInterval(interval);
  }, [fetchMilestone]);

  // Derive role information
  const isReviewer = grant?.reviewers.includes(address ?? "") ?? false;
  const isRecipient = grant?.recipient === address;

  // Derive vote information
  const hasVoted = votes.some((v) => v.reviewer === address && v.vote !== null);
  const currentVote = votes.find((v) => v.reviewer === address)?.vote === "approve" ? true : votes.find((v) => v.reviewer === address)?.vote === "reject" ? false : null;
  const approvalCount = votes.filter((v) => v.vote === "approve").length;
  const rejectionCount = votes.filter((v) => v.vote === "reject").length;
  const quorum = grant?.reviewers.length ?? 0;

  return {
    milestone,
    votes,
    isReviewer,
    isRecipient,
    hasVoted,
    currentVote,
    quorum,
    approvalCount,
    rejectionCount,
    isLoading,
    error,
    refetch: fetchMilestone,
  };
}
