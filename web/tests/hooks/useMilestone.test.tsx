import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/store/walletStore", () => ({
  useWalletStore: () => ({ address: "GREVIEWER1" }),
}));

import { useMilestone } from "@/hooks/useMilestone";

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

function mockFetch(reviewers: string[]) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      if (String(url).includes("/grant_reviewers/")) {
        return {
          ok: true,
          json: async () => ({
            data: reviewers.map((r) => ({ reviewerStellarAddress: r })),
          }),
        };
      }
      return {
        ok: true,
        json: async () => ({
          data: { id: 1, recipient: "GRECIPIENT", milestones: [{ idx: 0, votes: [] }] },
        }),
      };
    })
  );
}

describe("useMilestone", () => {
  beforeEach(() => vi.unstubAllGlobals());

  it("computes isReviewer and quorum from the reviewers endpoint", async () => {
    mockFetch(["GREVIEWER1", "GREVIEWER2", "GREVIEWER3"]);
    const { result } = renderHook(() => useMilestone("1", 0), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.error).toBeNull();
    expect(result.current.isReviewer).toBe(true);
    expect(result.current.quorum).toBe(3);
  });

  it("does not crash and is not a reviewer when the address is absent", async () => {
    mockFetch(["GOTHER"]);
    const { result } = renderHook(() => useMilestone("1", 0), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.isReviewer).toBe(false);
    expect(result.current.quorum).toBe(1);
  });
});
