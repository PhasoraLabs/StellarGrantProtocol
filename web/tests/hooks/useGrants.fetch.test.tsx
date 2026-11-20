/**
 * Issue #1287 — the grants-list fetch must succeed against the mock server.
 *
 * The mock route responds with the `GrantPage` envelope; this exercises the
 * real `useGrants` query path (fetch → parse → TanStack Query) and asserts it
 * no longer throws on `payload.grants`.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { useGrants } from "@/hooks/useGrants";
import { grants } from "@/mock-server/fixtures/grants";

/** The exact payload `mock-server/routes/grants.ts` sends over the wire. */
function mockListPayload(pageSize = 12) {
  return {
    grants: grants.slice(0, pageSize).map((grant) => ({
      ...grant,
      budget: grant.budget.toString(),
      funded: grant.funded.toString(),
      deadline: grant.deadline.toString(),
      created_at: grant.created_at.toString(),
    })),
    total: grants.length,
    nextPage: 2,
  };
}

function respondWith(payload: unknown) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => new Response(JSON.stringify(payload), { status: 200 })),
  );
}

function renderUseGrants() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return renderHook(() => useGrants({ page: 1 }), { wrapper });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("useGrants list fetch (#1287)", () => {
  it("succeeds against the mock server payload without throwing", async () => {
    respondWith(mockListPayload());

    const { result } = renderUseGrants();

    await waitFor(() => expect(result.current.data).not.toBeNull());

    expect(result.current.error).toBeNull();
    expect(result.current.data?.grants).toHaveLength(12);
    expect(result.current.data?.total).toBe(47);
    expect(result.current.data?.nextPage).toBe(2);
    expect(result.current.hasNextPage).toBe(true);

    // JSON cannot carry bigint — the hook revives the serialized fields.
    const first = result.current.data!.grants[0];
    expect(typeof first.budget).toBe("bigint");
    expect(typeof first.deadline).toBe("bigint");
    expect(first.budget).toBe(grants[0].budget);
    expect(fetch).toHaveBeenCalledWith("/api/grants?page=1");
  });

  it("still parses the legacy { data } envelope", async () => {
    const payload = mockListPayload();
    respondWith({ data: payload.grants, total: payload.total, nextPage: null });

    const { result } = renderUseGrants();

    await waitFor(() => expect(result.current.data).not.toBeNull());

    expect(result.current.error).toBeNull();
    expect(result.current.data?.grants).toHaveLength(12);
    expect(result.current.hasNextPage).toBe(false);
  });

  it("reports a query error instead of crashing when the payload is wrong", async () => {
    respondWith({ unexpected: "shape" });

    const { result } = renderUseGrants();

    await waitFor(() => expect(result.current.error).not.toBeNull());

    expect(result.current.error?.message).toMatch(/grants/i);
    expect(result.current.data).toBeNull();
    expect(result.current.errorType).toBe("generic");
  });
});
