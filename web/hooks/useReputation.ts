"use client";

/**
 * useReputation Hook
 *
 * Fetches contributor reputation from both the Soroban contract and the
 * indexing API, preferring API data when available.
 * Backed by TanStack Query (stale time: 2 min).
 * Return shape is unchanged for backward compatibility.
 */

import { useQuery } from "@tanstack/react-query";
import { contractClient } from "@/lib/stellar/contract";
import { api } from "@/lib/api";

export interface ReputationScoreInfo {
  score: number | null;
  grantsCompleted: number;
  totalEarned: bigint;
  tooltipContent: string;
}

interface UseReputationResult {
  score: number | null;
  grantsCompleted: number;
  totalEarned: bigint;
  isLoading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
  scoreInfo: ReputationScoreInfo;
}

interface ReputationData {
  score: number | null;
  grantsCompleted: number;
  totalEarned: bigint;
}

async function fetchReputation(address: string): Promise<ReputationData> {
  const [contributorResult, apiResult] = await Promise.allSettled([
    contractClient.contributorScore({ address }),
    api.get(`/contributors/${address}`),
  ]);

  if (contributorResult.status === "rejected" && apiResult.status === "rejected") {
    throw new Error("Failed to fetch reputation from all sources");
  }

  let score: number | null = null;
  let grantsCompleted = 0;
  let totalEarned = BigInt(0);

  if (contributorResult.status === "fulfilled") {
    score = Number(contributorResult.value);
  }

  if (apiResult.status === "fulfilled") {
    score = apiResult.value.data.reputation ?? score;
    grantsCompleted = apiResult.value.data.grants_participated ?? 0;
    totalEarned = BigInt(apiResult.value.data.total_earned ?? 0);
  }

  return { score, grantsCompleted, totalEarned };
}

function getScoreTooltip(score: number | null, grantsCompleted: number, totalEarned: bigint): string {
  if (score === null) {
    return "Reputation score not yet calculated. Complete grants to earn reputation.";
  }

  const earnedDisplay = totalEarned > 0 ? `${totalEarned.toString()} XLM` : "No earnings yet";
  return `Reputation Score: ${score}\n\nCalculated from:\n• Grants completed: ${grantsCompleted}\n• Total earned: ${earnedDisplay}\n\nYour score increases as you successfully complete grants and build trust in the community.`;
}

export function useReputation(address: string | null): UseReputationResult {
  const { data, isLoading, error, refetch } = useQuery<ReputationData, Error>({
    queryKey: ["reputation", address],
    queryFn: () => fetchReputation(address!),
    enabled: !!address,
    staleTime: 2 * 60_000, // 2 minutes
  });

  const score = data?.score ?? null;
  const grantsCompleted = data?.grantsCompleted ?? 0;
  const totalEarned = data?.totalEarned ?? BigInt(0);

  return {
    score,
    grantsCompleted,
    totalEarned,
    isLoading,
    error: error ?? null,
    refetch: async () => {
      await refetch();
    },
    scoreInfo: {
      score,
      grantsCompleted,
      totalEarned,
      tooltipContent: getScoreTooltip(score, grantsCompleted, totalEarned),
    },
  };
}
