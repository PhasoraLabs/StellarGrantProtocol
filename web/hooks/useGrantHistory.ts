"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getGrantHistory } from "@/lib/stellar/history";
import type { GrantHistoryRecord } from "@/lib/stellar/history";

const POLL_INTERVAL_MS = 60_000;

export interface TimelineGrouped {
  date: string;
  records: GrantHistoryRecord[];
}

export interface UseGrantHistoryResult {
  records: GrantHistoryRecord[];
  isLoading: boolean;
  error: Error | null;
  hasMore: boolean;
  loadMore: () => Promise<void>;
  refetch: () => Promise<void>;
  groupedByDate: TimelineGrouped[];
}

function parseGrantId(grantId: string): number | bigint {
  const asNumber = Number(grantId);
  if (!Number.isNaN(asNumber) && String(asNumber) === grantId) {
    return asNumber;
  }
  try {
    return BigInt(grantId);
  } catch {
    return grantId as unknown as number;
  }
}

function mergeRecords(
  existing: GrantHistoryRecord[],
  incoming: GrantHistoryRecord[],
  append: boolean,
): GrantHistoryRecord[] {
  const seen = new Set(existing.map((r) => r.txHash));
  const uniqueIncoming = incoming.filter((r) => !seen.has(r.txHash));
  if (uniqueIncoming.length === 0) return existing;
  return append ? [...existing, ...uniqueIncoming] : [...uniqueIncoming, ...existing];
}

export function useGrantHistory(grantId: string): UseGrantHistoryResult {
  const [records, setRecords] = useState<GrantHistoryRecord[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const cursorRef = useRef<string | undefined>(undefined);
  const loadMoreInFlightRef = useRef(false);
  const activeFetchIdRef = useRef(0);

  const fetchFirstPage = useCallback(
    async (options?: { showLoading?: boolean; mergeNew?: boolean }) => {
      if (!grantId) return;
      const { showLoading = true, mergeNew = false } = options ?? {};
      const fetchId = ++activeFetchIdRef.current;
      const targetGrantId = grantId;

      if (showLoading) queueMicrotask(() => setIsLoading(true));
      queueMicrotask(() => setError(null));

      await Promise.resolve();
      try {
        const { records: page, nextCursor } = await getGrantHistory(parseGrantId(targetGrantId));
        if (fetchId !== activeFetchIdRef.current || targetGrantId !== grantId) {
          return;
        }
        cursorRef.current = nextCursor;
        setHasMore(Boolean(nextCursor));
        setRecords((prev) =>
          mergeNew ? mergeRecords(prev, page, false) : page,
        );
      } catch (err) {
        if (fetchId !== activeFetchIdRef.current || targetGrantId !== grantId) {
          return;
        }
        const nextError = err instanceof Error ? err : new Error(String(err));
        setError(nextError);
        if (!mergeNew) {
          setRecords([]);
          setHasMore(false);
          cursorRef.current = undefined;
        }
      } finally {
        if (fetchId === activeFetchIdRef.current && targetGrantId === grantId) {
          if (showLoading) setIsLoading(false);
        }
      }
    },
    [grantId],
  );

  const refetch = useCallback(async () => {
    cursorRef.current = undefined;
    await fetchFirstPage({ showLoading: true, mergeNew: false });
  }, [fetchFirstPage]);

  const loadMore = useCallback(async () => {
    const cursor = cursorRef.current;
    if (!grantId || !cursor || loadMoreInFlightRef.current) return;
    const targetGrantId = grantId;

    loadMoreInFlightRef.current = true;
    setError(null);

    try {
      const { records: page, nextCursor } = await getGrantHistory(parseGrantId(targetGrantId), {
        cursor,
      });
      if (targetGrantId !== grantId) return;
      cursorRef.current = nextCursor;
      setHasMore(Boolean(nextCursor));
      setRecords((prev) => mergeRecords(prev, page, true));
    } catch (err) {
      if (targetGrantId !== grantId) return;
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      loadMoreInFlightRef.current = false;
    }
  }, [grantId]);

  useEffect(() => {
    cursorRef.current = undefined;
    const fetchIdTracker = activeFetchIdRef;
    queueMicrotask(() => {
      void fetchFirstPage();
    });

    const pollId = setInterval(() => {
      void fetchFirstPage({ showLoading: false, mergeNew: true });
    }, POLL_INTERVAL_MS);

    return () => {
      fetchIdTracker.current++;
      clearInterval(pollId);
    };
  }, [fetchFirstPage]);

  const groupedByDate = useMemo<TimelineGrouped[]>(() => {
    if (!records.length) return [];

    const groups: Record<string, GrantHistoryRecord[]> = {};
    records.forEach((record) => {
      const date = new Date(record.createdAt).toLocaleDateString("en-US", {
        year: "numeric",
        month: "long",
        day: "numeric",
      });
      if (!groups[date]) {
        groups[date] = [];
      }
      groups[date].push(record);
    });

    return Object.entries(groups)
      .sort(([dateA], [dateB]) => new Date(dateB).getTime() - new Date(dateA).getTime())
      .map(([date, recordsList]) => ({
        date,
        records: recordsList.sort(
          (a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime(),
        ),
      }));
  }, [records]);

  return {
    records,
    isLoading,
    error,
    hasMore,
    loadMore,
    refetch,
    groupedByDate,
  };
}
