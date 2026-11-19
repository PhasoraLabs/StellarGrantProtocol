"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { FunderRow } from "@/lib/grants/api";

export const FUNDERS_PAGE_SIZE = 10;

export function useFunders(grantId: string, pageSize = FUNDERS_PAGE_SIZE) {
  const [funders, setFunders] = useState<FunderRow[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [visibleCount, setVisibleCount] = useState(pageSize);

  const abortControllerRef = useRef<AbortController | null>(null);
  const fetchIdRef = useRef(0);

  const fetchFunders = useCallback(async () => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }

    const controller = new AbortController();
    abortControllerRef.current = controller;
    const currentFetchId = ++fetchIdRef.current;
    const targetGrantId = grantId;

    // Defer state updates to microtask to avoid sync-setState-in-effect warning
    await Promise.resolve();

    if (controller.signal.aborted || currentFetchId !== fetchIdRef.current) {
      return;
    }

    setIsLoading(true);

    if (!targetGrantId) {
      setFunders([]);
      setIsLoading(false);
      return;
    }

    try {
      const res = await fetch(`/api/grants/${targetGrantId}/funders`, {
        signal: controller.signal,
      });

      if (
        controller.signal.aborted ||
        currentFetchId !== fetchIdRef.current ||
        targetGrantId !== grantId
      ) {
        return;
      }

      if (!res.ok) throw new Error("Failed to load funders");
      const json = (await res.json()) as {
        funders: Array<{ address: string; amount: string }>;
      };

      if (
        controller.signal.aborted ||
        currentFetchId !== fetchIdRef.current ||
        targetGrantId !== grantId
      ) {
        return;
      }

      setFunders(
        (json.funders ?? []).map((f) => ({
          address: f.address,
          amount: BigInt(f.amount),
        }))
      );
      setVisibleCount(pageSize);
    } catch (err) {
      if (
        controller.signal.aborted ||
        (err as Error)?.name === "AbortError" ||
        currentFetchId !== fetchIdRef.current ||
        targetGrantId !== grantId
      ) {
        return;
      }
      setFunders([]);
    } finally {
      if (
        !controller.signal.aborted &&
        currentFetchId === fetchIdRef.current &&
        targetGrantId === grantId
      ) {
        setIsLoading(false);
      }
    }
  }, [grantId, pageSize]);

  const loadMore = useCallback(() => {
    setVisibleCount((count) => count + pageSize);
  }, [pageSize]);

  useEffect(() => {
    queueMicrotask(() => {
      void fetchFunders();
    });

    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
      }
    };
  }, [fetchFunders]);

  return {
    funders,
    visibleFunders: funders.slice(0, visibleCount),
    hasMore: funders.length > visibleCount,
    loadMore,
    isLoading,
    refetch: fetchFunders,
  };
}

