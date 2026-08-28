"use client";

import { useCallback, useEffect, useState } from "react";
import type { FunderRow } from "@/lib/grants/api";

export function useFunders(grantId: string) {
  const [funders, setFunders] = useState<FunderRow[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const fetchFunders = useCallback(async () => {
    // Defer state updates to microtask to avoid sync-setState-in-effect warning
    await Promise.resolve();
    setIsLoading(true);
    try {
      const res = await fetch(`/api/grants/${grantId}/funders`);
      if (!res.ok) throw new Error("Failed to load funders");
      const json = (await res.json()) as {
        funders: Array<{ address: string; amount: string }>;
      };
      setFunders(
        (json.funders ?? []).map((f) => ({
          address: f.address,
          amount: BigInt(f.amount),
        }))
      );
    } catch {
      setFunders([]);
    } finally {
      setIsLoading(false);
    }
  }, [grantId]);

  useEffect(() => {
    queueMicrotask(() => {
      void fetchFunders();
    });
  }, [fetchFunders]);

  return { funders, isLoading, refetch: fetchFunders };
}
