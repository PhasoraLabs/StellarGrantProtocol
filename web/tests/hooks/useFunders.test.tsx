import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { act, render, renderHook, screen, waitFor } from "@testing-library/react";
import { FUNDERS_PAGE_SIZE, useFunders } from "@/hooks/useFunders";

function FundersProbe({ grantId }: { grantId: string }) {
  const { funders, isLoading, refetch } = useFunders(grantId);
  return (
    <div>
      <span data-testid="loading">{String(isLoading)}</span>
      <span data-testid="count">{funders.length}</span>
      <button type="button" onClick={() => void refetch()}>
        Refetch
      </button>
      <ul>
        {funders.map((f) => (
          <li key={f.address} data-testid={`funder-${f.address}`}>
            {f.address}: {f.amount.toString()}
          </li>
        ))}
      </ul>
    </div>
  );
}

describe("useFunders", () => {
  const originalFetch = global.fetch;

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    global.fetch = originalFetch;
  });

  it("fetches and renders funders for a grant", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        funders: [
          { address: "GABC1", amount: "50000000" },
          { address: "GDEF2", amount: "100000000" },
        ],
      }),
    });

    render(<FundersProbe grantId="10" />);

    await waitFor(() => {
      expect(screen.getByTestId("loading").textContent).toBe("false");
      expect(screen.getByTestId("count").textContent).toBe("2");
      expect(screen.getByTestId("funder-GABC1").textContent).toContain("50000000");
      expect(screen.getByTestId("funder-GDEF2").textContent).toContain("100000000");
    });

    expect(global.fetch).toHaveBeenCalledWith(
      "/api/grants/10/funders",
      expect.objectContaining({ signal: expect.any(AbortSignal) }),
    );
  });

  it("rapidly changing grantId with out-of-order resolution confirms final state is correct", async () => {
    let resolveGrant1!: (val: unknown) => void;
    const delayedPromise1 = new Promise((resolve) => {
      resolveGrant1 = resolve;
    });

    global.fetch = vi.fn().mockImplementation((url: string) => {
      if (url.includes("/api/grants/1/funders")) {
        return delayedPromise1;
      }
      if (url.includes("/api/grants/2/funders")) {
        return Promise.resolve({
          ok: true,
          json: async () => ({
            funders: [{ address: "GGRANT2", amount: "2000000" }],
          }),
        });
      }
      return Promise.resolve({ ok: false, status: 404 });
    });

    const { rerender } = render(<FundersProbe grantId="1" />);

    // Rapidly switch to grant 2
    rerender(<FundersProbe grantId="2" />);

    // Grant 2 resolves first
    await waitFor(() => {
      expect(screen.getByTestId("count").textContent).toBe("1");
      expect(screen.getByTestId("funder-GGRANT2")).toBeTruthy();
    });

    // Grant 1 resolves late
    resolveGrant1({
      ok: true,
      json: async () => ({
        funders: [{ address: "GGRANT1_STALE", amount: "1000000" }],
      }),
    });

    // Wait and verify grant 1 never stomped grant 2's data
    await new Promise((r) => setTimeout(r, 50));
    expect(screen.queryByTestId("funder-GGRANT1_STALE")).toBeNull();
    expect(screen.getByTestId("funder-GGRANT2")).toBeTruthy();
  });

  it("handles fetch errors gracefully by setting funders to empty array", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
    });

    render(<FundersProbe grantId="99" />);

    await waitFor(() => {
      expect(screen.getByTestId("loading").textContent).toBe("false");
      expect(screen.getByTestId("count").textContent).toBe("0");
    });
  });

  it("aborts in-flight request on unmount", async () => {
    let capturedSignal!: AbortSignal;
    global.fetch = vi.fn().mockImplementation((_url: string, init?: RequestInit) => {
      capturedSignal = init?.signal as AbortSignal;
      return new Promise(() => {}); // never resolves
    });

    const { unmount } = render(<FundersProbe grantId="1" />);

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalled();
    });

    expect(capturedSignal.aborted).toBe(false);
    unmount();
    expect(capturedSignal.aborted).toBe(true);
  });

  it("shows funders one page at a time", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        funders: Array.from({ length: 25 }, (_, i) => ({
          address: `G${i}`,
          amount: "1",
        })),
      }),
    });

    const { result } = renderHook(() => useFunders("10"));

    await waitFor(() => expect(result.current.funders).toHaveLength(25));
    expect(result.current.visibleFunders).toHaveLength(FUNDERS_PAGE_SIZE);
    expect(result.current.hasMore).toBe(true);

    act(() => result.current.loadMore());
    act(() => result.current.loadMore());

    expect(result.current.visibleFunders).toHaveLength(25);
    expect(result.current.hasMore).toBe(false);
  });
});
