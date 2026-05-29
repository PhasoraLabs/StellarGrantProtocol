"use client";

import { useState, useEffect, useCallback } from "react";
import { contractClient } from "@/lib/stellar/contract";
import { api } from "@/lib/api";

interface UseReputationResult {
  score: number | null;
  grantsCompleted: number;
  totalEarned: bigint;
  isLoading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

interface ReputationData {
  score: number | null;
  grantsCompleted: number;
  totalEarned: bigint;
}

const cache = new Map<
  string,
  { data: ReputationData; timestamp: number }
>();
const CACHE_TTL = 60_000;

export function useReputation(address: string | null): UseReputationResult {
  const [score, setScore] = useState<number | null>(null);
  const [grantsCompleted, setGrantsCompleted] = useState(0);
  const [totalEarned, setTotalEarned] = useState<bigint>(BigInt(0));
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetch = useCallback(async () => {
    if (!address) {
      setScore(null);
      setGrantsCompleted(0);
      setTotalEarned(BigInt(0));
      setIsLoading(false);
      setError(null);
      return;
    }

    const cached = cache.get(address);
    if (cached && Date.now() - cached.timestamp < CACHE_TTL) {
      setScore(cached.data.score);
      setGrantsCompleted(cached.data.grantsCompleted);
      setTotalEarned(cached.data.totalEarned);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const [contributorResult, apiResult] = await Promise.allSettled([
        contractClient.contributorScore({ address }),
        api.get(`/contributors/${address}`),
      ]);

      let finalScore: number | null = null;
      let finalGrants = 0;
      let finalEarned = BigInt(0);

      if (contributorResult.status === "fulfilled") {
        finalScore = Number(contributorResult.value);
      }

      if (apiResult.status === "fulfilled") {
        finalScore = apiResult.value.data.reputation ?? finalScore;
        finalGrants = apiResult.value.data.grants_participated ?? 0;
        finalEarned = BigInt(apiResult.value.data.total_earned ?? 0);
      }

      if (
        contributorResult.status === "rejected" &&
        apiResult.status === "rejected"
      ) {
        throw new Error("Failed to fetch reputation from all sources");
      }

      cache.set(address, {
        data: {
          score: finalScore,
          grantsCompleted: finalGrants,
          totalEarned: finalEarned,
        },
        timestamp: Date.now(),
      });

      setScore(finalScore);
      setGrantsCompleted(finalGrants);
      setTotalEarned(finalEarned);
    } catch (err) {
      const e = err instanceof Error ? err : new Error("Unknown error");
      setError(e);
    } finally {
      setIsLoading(false);
    }
  }, [address]);

  useEffect(() => {
    fetch();
  }, [fetch]);

  return {
    score,
    grantsCompleted,
    totalEarned,
    isLoading,
    error,
    refetch: fetch,
  };
}
