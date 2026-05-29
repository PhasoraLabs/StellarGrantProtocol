import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { WatchButton } from "@/components/grants/WatchButton";
import { WATCHLIST_STORAGE_KEY } from "@/hooks/useWatchlist";

describe("WatchButton", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.restoreAllMocks();
  });

  it("renders without a connected wallet", () => {
    render(<WatchButton grantId="12" />);
    expect(screen.getByRole("button", { name: /watch grant/i })).toBeTruthy();
  });

  it("toggles to watching and dispatches a toast", () => {
    const dispatch = vi.spyOn(window, "dispatchEvent");

    render(<WatchButton grantId="12" />);
    fireEvent.click(screen.getByRole("button", { name: /watch grant/i }));

    expect(screen.getByRole("button", { name: /watching/i })).toBeTruthy();
    expect(JSON.parse(localStorage.getItem(WATCHLIST_STORAGE_KEY) ?? "[]")).toContain("12");
    expect(dispatch).toHaveBeenCalled();
  });
});
