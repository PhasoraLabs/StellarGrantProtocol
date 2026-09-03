"use client";

/**
 * useMyGrants Hook
 *
 * Lists grants owned or funded by the currently connected wallet address.
 * Backed by TanStack Query for caching and background refetch.
 * Return shape is unchanged for backward compatibility.
 */

import { useQuery } from "@tanstack/react-query";
import type { Grant } from "@/types";
import { logger } from "@/lib/logger";
import { useWalletStore } from "@/lib/store";

interface MyGrants {
  owned: Grant[];
  funded: Grant[];
}

interface UseMyGrantsResult {
  data: MyGrants | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

const hookLogger = logger.child("useMyGrants");

async function fetchMyGrants(address: string): Promise<MyGrants> {
  const [ownedRes, fundedRes] = await Promise.all([
    fetch(`/api/grants?owner=${address}`),
    fetch(`/api/grants?funder=${address}`),
  ]);

  if (!ownedRes.ok) throw new Error(`Failed to fetch owned grants: ${ownedRes.status}`);
  if (!fundedRes.ok) throw new Error(`Failed to fetch funded grants: ${fundedRes.status}`);

  const [ownedJson, fundedJson] = await Promise.all([
    ownedRes.json() as Promise<{ grants: Grant[] }>,
    fundedRes.json() as Promise<{ grants: Grant[] }>,
  ]);

  hookLogger.debug("My grants fetched", {
    owned: ownedJson.grants.length,
    funded: fundedJson.grants.length,
  });

  return { owned: ownedJson.grants, funded: fundedJson.grants };
}

export function useMyGrants(): UseMyGrantsResult {
  const address = useWalletStore((s) => s.address);

  const { data, isLoading, error, refetch } = useQuery<MyGrants, Error>({
    queryKey: ["my-grants", address],
    queryFn: () => fetchMyGrants(address!),
    enabled: !!address,
    staleTime: 30_000,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error ?? null,
    refetch: async () => {
      await refetch();
    },
  };
}
