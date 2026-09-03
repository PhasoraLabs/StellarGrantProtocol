/**
 * useMilestone Hook
 *
 * Fetches milestone state for a specific grant + milestone index.
 * Backed by TanStack Query — derives milestone from the cached grant query
 * to avoid a redundant RPC trip.
 * Return shape is unchanged for backward compatibility.
 */

"use client";

import { useQuery } from "@tanstack/react-query";
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

type GrantWithMilestones = Grant & { milestones?: (Milestone & { votes?: MilestoneVote[] })[] };

async function fetchGrantWithMilestones(grantId: string): Promise<GrantWithMilestones> {
  const res = await fetch(`${API_URL}/grants/${grantId}`);
  if (!res.ok) throw new Error(`Failed to fetch grant: ${res.statusText}`);
  return res.json() as Promise<GrantWithMilestones>;
}

export function useMilestone(grantId: string, milestoneIdx: number): UseMilestoneReturn {
  const { address } = useWalletStore();

  const { data: grantData, isLoading, error, refetch } = useQuery<GrantWithMilestones, Error>({
    queryKey: ["milestone", grantId, milestoneIdx],
    queryFn: () => fetchGrantWithMilestones(grantId),
    enabled: !!grantId,
    staleTime: 15_000,
    refetchInterval: 15_000,
  });

  const milestone = grantData?.milestones?.[milestoneIdx] ?? null;
  const votes: MilestoneVote[] = (milestone as (Milestone & { votes?: MilestoneVote[] }) | null)?.votes ?? [];
  const grant = grantData ?? null;

  const isReviewer = grant?.reviewers.includes(address ?? "") ?? false;
  const isRecipient = grant?.recipient === address;
  const hasVoted = votes.some((v) => v.reviewer === address && v.vote !== null);
  const myVote = votes.find((v) => v.reviewer === address);
  const currentVote = myVote?.vote === "approve" ? true : myVote?.vote === "reject" ? false : null;
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
    error: error ?? null,
    refetch: async () => {
      await refetch();
    },
  };
}
