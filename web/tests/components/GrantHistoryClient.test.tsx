import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { GrantHistoryClient } from "@/app/grants/[id]/history/GrantHistoryClient";
import type { GrantHistoryRecord } from "@/lib/stellar/history";

const mockUseGrantHistory = vi.fn();

vi.mock("@/hooks/useGrantHistory", () => ({
  useGrantHistory: (grantId: string) => mockUseGrantHistory(grantId),
}));

vi.mock("@/hooks/useRelativeTime", () => ({
  useRelativeTime: () => "2h ago",
}));

function makeRecord(overrides: Partial<GrantHistoryRecord> & { txHash: string }): GrantHistoryRecord {
  return {
    createdAt: "2026-01-01T12:00:00Z",
    successful: true,
    operationType: "grant_fund",
    sourceAccount: `G${"A".repeat(55)}`,
    feeCharged: "100",
    grantId: "42",
    memo: "+500 XLM",
    ...overrides,
  };
}

describe("GrantHistoryClient", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseGrantHistory.mockReturnValue({
      records: [],
      isLoading: false,
      error: null,
      hasMore: false,
      loadMore: vi.fn(),
      refetch: vi.fn(),
    });
  });

  it("shows empty state when no records exist", () => {
    render(<GrantHistoryClient grantId="42" />);
    expect(
      screen.getByText("No on-chain activity recorded yet for this grant."),
    ).toBeTruthy();
  });

  it("shows operation labels, explorer links, and load more", async () => {
    const loadMore = vi.fn();
    mockUseGrantHistory.mockReturnValue({
      records: [
        makeRecord({
          txHash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
          operationType: "grant_fund",
        }),
        makeRecord({
          txHash: "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
          operationType: "milestone_approve",
          memo: "Phase 1",
        }),
      ],
      isLoading: false,
      error: null,
      hasMore: true,
      loadMore,
      refetch: vi.fn(),
    });

    render(<GrantHistoryClient grantId="42" />);

    expect(screen.getByText("Funding Deposit")).toBeTruthy();
    expect(screen.getByText("Milestone Approved")).toBeTruthy();
    expect(screen.getByText("+500 XLM")).toBeTruthy();
    expect(screen.getByText("✓ Phase 1")).toBeTruthy();

    const txHash =
      "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    const explorerLink = screen.getByTitle(txHash);
    expect(explorerLink.getAttribute("href")).toContain(txHash);

    fireEvent.click(screen.getByRole("button", { name: /load more/i }));
    await waitFor(() => {
      expect(loadMore).toHaveBeenCalled();
    });
  });

  it("links back to grant detail and calls refetch on refresh", () => {
    const refetch = vi.fn();
    mockUseGrantHistory.mockReturnValue({
      records: [makeRecord({ txHash: "tx1" })],
      isLoading: false,
      error: null,
      hasMore: false,
      loadMore: vi.fn(),
      refetch,
    });

    render(<GrantHistoryClient grantId="42" />);

    expect(screen.getByRole("link", { name: /back to grant/i }).getAttribute("href")).toBe(
      "/grants/42",
    );

    fireEvent.click(screen.getByRole("button", { name: /refresh/i }));
    expect(refetch).toHaveBeenCalled();
  });
});
