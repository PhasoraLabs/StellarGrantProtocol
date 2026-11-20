"use client";

/**
 * useGrantBalances Hook
 *
 * Fetches and monitors XLM and SAC token balances held by a grant's
 * contract account. Backed by TanStack Query (stale time: 10 s,
 * background refetchInterval matches the caller-supplied pollInterval).
 * The `onChange` callback fires whenever the data changes.
 * Return shape is unchanged for backward compatibility.
 */

import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  getGrantBalances,
  type GrantBalances,
  type GrantBalance,
} from "@/lib/stellar/balances";
import { logger } from "@/lib/logger";

const hookLogger = logger.child("useGrantBalances");

interface UseGrantBalancesOptions {
  contractAddress: string;
  pollInterval?: number;
  enabled?: boolean;
  onChange?: (current: GrantBalances, previous: GrantBalances | null) => void;
}

interface UseGrantBalancesResult {
  balances: GrantBalances | null;
  balanceList: GrantBalance[];
  xlmBalance: GrantBalance | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
  lastUpdated: Date | null;
}

export function useGrantBalances(
  options: UseGrantBalancesOptions
): UseGrantBalancesResult {
  const { contractAddress, pollInterval = 15_000, enabled = true, onChange } = options;
  const previousRef = useRef<GrantBalances | null>(null);

  const { data, isLoading, error, refetch } = useQuery<GrantBalances, Error>({
    queryKey: ["balances", contractAddress],
    queryFn: async () => {
      hookLogger.debug("Fetching grant balances", { contractAddress });
      const snapshot = await getGrantBalances(contractAddress);
      hookLogger.debug("Balances fetched", { contractAddress, count: snapshot.balances.length });
      return snapshot;
    },
    enabled: enabled && !!contractAddress,
    staleTime: 10_000,
    refetchInterval: enabled ? pollInterval : false,
  });

  // Fire onChange when data changes (TanStack Query already dedupes re-renders)
  useEffect(() => {
    if (!data || !onChange) return;
    if (data !== previousRef.current) {
      onChange(data, previousRef.current);
      previousRef.current = data;
    }
  }, [data, onChange]);

  const balanceList = data?.balances ?? [];
  const xlmBalance = balanceList.find((b) => b.isNative) ?? null;

  return {
    balances: data ?? null,
    balanceList,
    xlmBalance,
    isLoading,
    error: error ?? null,
    refetch: async () => {
      await refetch();
    },
    lastUpdated: data?.fetchedAt ?? null,
  };
}
