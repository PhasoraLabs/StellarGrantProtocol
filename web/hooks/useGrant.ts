"use client";

/**
 * useGrant Hook — fetches grant detail with milestones.
 * Backed by TanStack Query for automatic caching, background refetch,
 * and deduplication. Return shape is unchanged for backward compatibility.
 */

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { notFound } from "next/navigation";
import type { Grant, Milestone } from "@/types";
import { logger } from "@/lib/logger";

interface UseGrantOptions {
  /** How often to background-refetch in ms. Default: 15 000. */
  refetchInterval?: number;
  enabled?: boolean;
}

export interface GrantDetailData {
  grant: Grant;
  milestones: Milestone[];
  completedMilestones: number;
  isWatched: boolean;
}

interface UseGrantResult {
  data: GrantDetailData | null;
  isLoading: boolean;
  error: Error | null;
  errorType: "network" | "api" | "rpc" | "generic";
  refetch: () => Promise<void>;
}

const hookLogger = logger.child("useGrant");

async function fetchGrantDetail(grantId: string): Promise<GrantDetailData> {
  const res = await fetch(`/api/grants/${grantId}`);
  if (res.status === 404) notFound();
  if (!res.ok) throw new Error(`Failed to fetch grant ${grantId}: ${res.status}`);

  const json = (await res.json()) as {
    grant: Grant;
    milestones: Milestone[];
    completedMilestones: number;
    isWatched: boolean;
    reviewers?: string[];
  };

  hookLogger.debug("Grant fetched", { grantId, status: json.grant?.status });

  const grant: Grant = {
    ...json.grant,
    budget: BigInt(json.grant.budget),
    funded: BigInt(json.grant.funded),
    deadline: BigInt(json.grant.deadline),
    created_at: BigInt(json.grant.created_at),
    reviewers: json.reviewers ?? json.grant.reviewers,
  };

  return {
    grant,
    milestones: json.milestones ?? [],
    completedMilestones: json.completedMilestones ?? 0,
    isWatched: json.isWatched ?? false,
  };
}

function classifyError(err: Error): "network" | "api" | "rpc" | "generic" {
  if (err.message.includes("Failed to fetch") || (typeof navigator !== "undefined" && !navigator.onLine)) {
    return "network";
  }
  if (err.message.includes("503") || err.message.includes("504")) return "api";
  return "generic";
}

export function useGrant(grantId: string, options?: UseGrantOptions): UseGrantResult {
  const { refetchInterval = 15_000, enabled = true } = options ?? {};
  const queryClient = useQueryClient();

  const { data, isLoading, error, refetch } = useQuery<GrantDetailData, Error>({
    queryKey: ["grant", grantId],
    queryFn: () => {
      hookLogger.debug("Fetching grant", { grantId });
      return fetchGrantDetail(grantId);
    },
    enabled: enabled && !!grantId,
    staleTime: 15_000,
    refetchInterval: enabled ? refetchInterval : false,
  });

  // Prefetch on hover support: expose queryClient.prefetchQuery for GrantCard
  void queryClient; // used by callers via useQueryClient directly

  return {
    data: data ?? null,
    isLoading,
    error: error ?? null,
    errorType: error ? classifyError(error) : "generic",
    refetch: async () => {
      await refetch();
    },
  };
}
