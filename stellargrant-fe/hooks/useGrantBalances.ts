"use client";

/**
 * useGrantBalances Hook
 *
 * React hook that fetches and monitors the real-time XLM and SAC token
 * balances held by the smart contract account of a given grant.
 *
 * Features:
 * - Initial fetch on mount
 * - Configurable polling interval (default: 15s)
 * - Automatic cleanup on unmount
 * - Balance-change detection (only triggers re-render when values actually change)
 */

import { useState, useEffect, useCallback, useRef } from "react";
import {
  getGrantBalances,
  listenToBalanceChanges,
  type GrantBalances,
  type GrantBalance,
} from "@/lib/stellar/balances";
import { logger } from "@/lib/logger";

const hookLogger = logger.child("useGrantBalances");

// ── Options & Result types ─────────────────────────────────────────────────

interface UseGrantBalancesOptions {
  /** Contract address of the grant account. Hook is disabled if empty. */
  contractAddress: string;
  /** How often to poll for balance changes (ms). Default: 15_000. */
  pollInterval?: number;
  /** Set to false to pause fetching. Default: true. */
  enabled?: boolean;
  /** Called when a balance change is detected during polling */
  onChange?: (current: GrantBalances, previous: GrantBalances | null) => void;
}

interface UseGrantBalancesResult {
  /** Full balance snapshot (null while loading or on error) */
  balances: GrantBalances | null;
  /** Individual balance entries for rendering convenience */
  balanceList: GrantBalance[];
  /** Shortcut: native XLM balance entry */
  xlmBalance: GrantBalance | null;
  /** True during the first fetch */
  isLoading: boolean;
  /** Any error from the most recent fetch attempt */
  error: Error | null;
  /** Manually trigger a balance refresh */
  refetch: () => Promise<void>;
  /** Timestamp of the last successful fetch */
  lastUpdated: Date | null;
}

// ── Hook ───────────────────────────────────────────────────────────────────

export function useGrantBalances(
  options: UseGrantBalancesOptions
): UseGrantBalancesResult {
  const { contractAddress, pollInterval = 15_000, enabled = true, onChange: onBalanceChange } = options;

  const [balances, setBalances] = useState<GrantBalances | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const abortRef = useRef<AbortController | null>(null);

  // ── Manual fetch ─────────────────────────────────────────────────────────
  const fetch = useCallback(async () => {
    if (!enabled || !contractAddress) return;

    abortRef.current?.abort();
    abortRef.current = new AbortController();
    setIsLoading(true);
    setError(null);
    hookLogger.debug("Fetching grant balances", { contractAddress });

    try {
      const snapshot = await getGrantBalances(contractAddress);
      setBalances(snapshot);
      setLastUpdated(snapshot.fetchedAt);
      hookLogger.debug("Balances fetched", {
        contractAddress,
        count: snapshot.balances.length,
      });
    } catch (err) {
      if ((err as { name?: string }).name === "AbortError") return;
      const error = err instanceof Error ? err : new Error(String(err));
      hookLogger.error("Error fetching balances", {
        contractAddress,
        error: error.message,
      });
      setError(error);
    } finally {
      setIsLoading(false);
    }
  }, [contractAddress, enabled]);

  // ── Initial fetch + polling listener ─────────────────────────────────────
  useEffect(() => {
    if (!enabled || !contractAddress) return;

    // Initial fetch
    void fetch();

    // Then subscribe to changes via polling
    let previousSnapshot: GrantBalances | null = null;
    const stop = listenToBalanceChanges(contractAddress, {
      pollInterval,
      onChange: (current, previous) => {
        setBalances(current);
        setLastUpdated(current.fetchedAt);
        previousSnapshot = previous;
        onBalanceChange?.(current, previous ?? previousSnapshot);
        hookLogger.debug("Balance change detected", { contractAddress });
      },
      onError: (err) => {
        hookLogger.error("Balance listener error", {
          contractAddress,
          error: err.message,
        });
        setError(err);
      },
    });

    return () => {
      stop();
      abortRef.current?.abort();
    };
  }, [contractAddress, enabled, pollInterval, fetch, onBalanceChange]);

  // ── Derived values ────────────────────────────────────────────────────────
  const balanceList = balances?.balances ?? [];
  const xlmBalance = balanceList.find((b) => b.isNative) ?? null;

  return {
    balances,
    balanceList,
    xlmBalance,
    isLoading,
    error,
    refetch: fetch,
    lastUpdated,
  };
}
