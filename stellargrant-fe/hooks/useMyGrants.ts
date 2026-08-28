"use client";

/**
 * useMyGrants Hook
 *
 * Lists grants owned or funded by the currently connected wallet address.
 * Returns two separate arrays — owned and funded — with shared loading state.
 */

import { useState, useEffect, useCallback } from "react";
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

export function useMyGrants(): UseMyGrantsResult {
  const address = useWalletStore((s) => s.address);

  const [data, setData] = useState<MyGrants | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetch_ = useCallback(async () => {
    if (!address) {
      setData(null);
      return;
    }

    queueMicrotask(() => setIsLoading(true));
    queueMicrotask(() => setError(null));

    await Promise.resolve();
    try {
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

      setData({ owned: ownedJson.grants, funded: fundedJson.grants });
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      hookLogger.error("Error fetching my grants", { error: error.message });
      setError(error);
    } finally {
      setIsLoading(false);
    }
  }, [address]);

  useEffect(() => {
    queueMicrotask(() => {
      void fetch_();
    });
  }, [fetch_]);

  return { data, isLoading, error, refetch: fetch_ };
}
