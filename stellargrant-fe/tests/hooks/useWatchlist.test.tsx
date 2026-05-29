import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import {
  useWatchlist,
  WATCHLIST_STORAGE_KEY,
  WATCHLIST_MAX_ITEMS,
} from "@/hooks/useWatchlist";

describe("useWatchlist", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("persists watched grant ids to localStorage", () => {
    const { result } = renderHook(() => useWatchlist());

    act(() => {
      result.current.add("42");
    });

    expect(result.current.isWatched("42")).toBe(true);
    expect(JSON.parse(localStorage.getItem(WATCHLIST_STORAGE_KEY) ?? "[]")).toContain("42");
  });

  it("toggle removes an existing id", () => {
    const { result } = renderHook(() => useWatchlist());

    act(() => {
      result.current.add("7");
      result.current.toggle("7");
    });

    expect(result.current.isWatched("7")).toBe(false);
  });

  it("caps the watchlist at 100 items, dropping the oldest", () => {
    const { result } = renderHook(() => useWatchlist());

    act(() => {
      for (let i = 0; i < WATCHLIST_MAX_ITEMS + 1; i += 1) {
        result.current.add(String(i));
      }
    });

    expect(result.current.watchedIds).toHaveLength(WATCHLIST_MAX_ITEMS);
    expect(result.current.watchedIds[0]).toBe("1");
    expect(result.current.watchedIds.at(-1)).toBe(String(WATCHLIST_MAX_ITEMS));
  });

  it("hydrates from existing localStorage on mount", () => {
    localStorage.setItem(WATCHLIST_STORAGE_KEY, JSON.stringify(["99", "100"]));

    const { result } = renderHook(() => useWatchlist());

    expect(result.current.isWatched("99")).toBe(true);
    expect(result.current.watchedIds).toEqual(["99", "100"]);
  });
});
