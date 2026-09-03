import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { useGrantHistory } from "@/hooks/useGrantHistory";
import type { GrantHistoryRecord } from "@/lib/stellar/history";

const mockGetGrantHistory = vi.fn();

vi.mock("@/lib/stellar/history", () => ({
  getGrantHistory: (...args: unknown[]) => mockGetGrantHistory(...args),
}));

function makeRecord(overrides: Partial<GrantHistoryRecord> & { txHash: string }): GrantHistoryRecord {
  return {
    createdAt: "2026-01-01T12:00:00Z",
    successful: true,
    operationType: "grant_fund",
    sourceAccount: "GABC1234567890123456789012345678901234567890123456789012345678",
    feeCharged: "100",
    grantId: "1",
    memo: "grant:1",
    ...overrides,
  };
}

function HookProbe({ grantId }: { grantId: string }) {
  const result = useGrantHistory(grantId);
  return (
    <div>
      <span data-testid="count">{result.records.length}</span>
      <span data-testid="loading">{String(result.isLoading)}</span>
      <span data-testid="has-more">{String(result.hasMore)}</span>
      <span data-testid="error">{result.error?.message ?? ""}</span>
      <button type="button" onClick={() => void result.loadMore()}>
        Load more
      </button>
      <button type="button" onClick={() => void result.refetch()}>
        Refetch
      </button>
      <ul>
        {result.records.map((r) => (
          <li key={r.txHash}>{r.operationType}</li>
        ))}
      </ul>
    </div>
  );
}

describe("useGrantHistory", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetGrantHistory.mockResolvedValue({ records: [], nextCursor: undefined });
  });

  it("useGrantHistory('1') returns GrantHistoryRecord objects", async () => {
    const records = [
      makeRecord({ txHash: "tx-a", operationType: "grant_fund" }),
      makeRecord({ txHash: "tx-b", operationType: "milestone_submit" }),
    ];
    mockGetGrantHistory.mockResolvedValueOnce({ records, nextCursor: "cursor-1" });

    render(<HookProbe grantId="1" />);

    await waitFor(() => {
      expect(screen.getByTestId("count").textContent).toBe("2");
      expect(screen.getByText("grant_fund")).toBeTruthy();
      expect(screen.getByText("milestone_submit")).toBeTruthy();
    });

    expect(mockGetGrantHistory).toHaveBeenCalledWith(1);
    expect(screen.getByTestId("has-more").textContent).toBe("true");
  });

  it("loadMore() appends the next page of records", async () => {
    mockGetGrantHistory
      .mockResolvedValueOnce({
        records: [makeRecord({ txHash: "tx-1" })],
        nextCursor: "page-2",
      })
      .mockResolvedValueOnce({
        records: [makeRecord({ txHash: "tx-2", operationType: "milestone_payout" })],
        nextCursor: undefined,
      });

    render(<HookProbe grantId="1" />);

    await waitFor(() => {
      expect(screen.getByTestId("count").textContent).toBe("1");
    });

    fireEvent.click(screen.getByRole("button", { name: /load more/i }));

    await waitFor(() => {
      expect(screen.getByTestId("count").textContent).toBe("2");
      expect(screen.getByText("milestone_payout")).toBeTruthy();
    });

    expect(mockGetGrantHistory).toHaveBeenLastCalledWith(1, { cursor: "page-2" });
  });

  it("refetch replaces records from the first page", async () => {
    mockGetGrantHistory
      .mockResolvedValueOnce({
        records: [makeRecord({ txHash: "tx-old" })],
        nextCursor: undefined,
      })
      .mockResolvedValueOnce({
        records: [makeRecord({ txHash: "tx-new", operationType: "grant_cancel" })],
        nextCursor: undefined,
      });

    render(<HookProbe grantId="1" />);

    await waitFor(() => {
      expect(screen.getByText("grant_fund")).toBeTruthy();
    });

    fireEvent.click(screen.getByRole("button", { name: /refetch/i }));

    await waitFor(() => {
      expect(screen.queryByText("grant_fund")).toBeNull();
      expect(screen.getByText("grant_cancel")).toBeTruthy();
    });
  });
});
