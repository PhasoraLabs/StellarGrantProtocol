"use client";

import { useCallback, useEffect, useState } from "react";

export const WATCHLIST_STORAGE_KEY = "sg-watchlist";
export const WATCHLIST_MAX_ITEMS = 100;
export const WATCHLIST_UPDATED_EVENT = "stellar:watchlist-updated";

export interface UseWatchlistResult {
  watchedIds: string[];
  isWatched: (grantId: string) => boolean;
  toggle: (grantId: string) => void;
  add: (grantId: string) => void;
  remove: (grantId: string) => void;
  clear: () => void;
}

function readWatchlist(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(WATCHLIST_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((id): id is string => typeof id === "string" && id.length > 0);
  } catch {
    return [];
  }
}

function capWatchlist(ids: string[]): string[] {
  if (ids.length <= WATCHLIST_MAX_ITEMS) return ids;
  return ids.slice(ids.length - WATCHLIST_MAX_ITEMS);
}

function writeWatchlist(ids: string[]): string[] {
  const next = capWatchlist(ids);
  localStorage.setItem(WATCHLIST_STORAGE_KEY, JSON.stringify(next));
  window.dispatchEvent(new CustomEvent(WATCHLIST_UPDATED_EVENT));
  return next;
}

export function useWatchlist(): UseWatchlistResult {
  const [watchedIds, setWatchedIds] = useState<string[]>(readWatchlist);

  useEffect(() => {
    const sync = () => setWatchedIds(readWatchlist());
    window.addEventListener(WATCHLIST_UPDATED_EVENT, sync);
    const onStorage = (event: StorageEvent) => {
      if (event.key === WATCHLIST_STORAGE_KEY) sync();
    };
    window.addEventListener("storage", onStorage);
    return () => {
      window.removeEventListener(WATCHLIST_UPDATED_EVENT, sync);
      window.removeEventListener("storage", onStorage);
    };
  }, []);

  const persist = useCallback((updater: (prev: string[]) => string[]) => {
    const next = writeWatchlist(updater(readWatchlist()));
    setWatchedIds(next);
    return next;
  }, []);

  const add = useCallback(
    (grantId: string) => {
      const id = grantId.trim();
      if (!id) return;
      persist((prev) => {
        const without = prev.filter((entry) => entry !== id);
        return [...without, id];
      });
    },
    [persist],
  );

  const remove = useCallback(
    (grantId: string) => {
      persist((prev) => prev.filter((entry) => entry !== grantId));
    },
    [persist],
  );

  const toggle = useCallback(
    (grantId: string) => {
      const id = grantId.trim();
      if (!id) return;
      persist((prev) => {
        if (prev.includes(id)) return prev.filter((entry) => entry !== id);
        return [...prev.filter((entry) => entry !== id), id];
      });
    },
    [persist],
  );

  const clear = useCallback(() => {
    persist(() => []);
  }, [persist]);

  const isWatched = useCallback(
    (grantId: string) => watchedIds.includes(grantId),
    [watchedIds],
  );

  return {
    watchedIds,
    isWatched,
    toggle,
    add,
    remove,
    clear,
  };
}
