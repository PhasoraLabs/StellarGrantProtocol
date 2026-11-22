/**
 * Issue #1287 — the grants-list fetch must succeed against the mock server.
 *
 * The mock route responds with the `GrantPage` envelope; this exercises the
 * real `useGrants` query path (fetch → parse → TanStack Query) and asserts it
 * no longer throws on `payload.grants`.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { useGrants, buildGrantsUrl } from "@/hooks/useGrants";
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

  it("builds the grant URL with status filter and sort by amount", () => {
    const urlDesc = buildGrantsUrl(
      { status: "active", sort: "amount_desc", token: "USDC", q: "stellar" },
      1,
    );
    expect(urlDesc).toContain("status=active");
    expect(urlDesc).toContain("sort=amount_desc");
    expect(urlDesc).toContain("token=USDC");
    expect(urlDesc).toContain("q=stellar");
    expect(urlDesc).toContain("page=1");

    const urlAsc = buildGrantsUrl({ status: "completed", sort: "amount_asc" }, 2);
    expect(urlAsc).toContain("status=completed");
    expect(urlAsc).toContain("sort=amount_asc");
    expect(urlAsc).toContain("page=2");
  });

  it("exposes controls that update filters and sync to URL router", async () => {
    respondWith(mockListPayload());
    const pushMock = vi.fn();
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(
      () =>
        useGrants({
          syncWithUrl: true,
          router: { push: pushMock },
          searchParams: new URLSearchParams("status=open&sort=newest&page=1"),
        }),
      { wrapper },
    );

    expect(result.current.status).toBe("open");
    expect(result.current.sort).toBe("newest");

    // Change status
    act(() => {
      result.current.setStatus("active");
    });
    expect(result.current.status).toBe("active");
    expect(pushMock).toHaveBeenCalledWith("/grants?status=active&sort=newest");

    // Change sort to amount_desc
    act(() => {
      result.current.setSort("amount_desc");
    });
    expect(result.current.sort).toBe("amount_desc");
    expect(pushMock).toHaveBeenCalledWith("/grants?status=active&sort=amount_desc");

    // Reset filters
    act(() => {
      result.current.resetFilters();
    });
    expect(result.current.status).toBe("");
    expect(result.current.sort).toBe("newest");
    expect(pushMock).toHaveBeenCalledWith("/grants");
  });
});
