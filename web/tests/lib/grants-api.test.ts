import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { API_URL } from "@/lib/constants";
import {
  fetchGrantById,
  fetchGrantFunders,
  fetchGrantReviewers,
} from "@/lib/grants/api";

const fetchMock = vi.fn();

const grantDetail = {
  data: {
    id: 7,
    title: "Test grant",
    status: "active",
    recipient: "GCREATED",
    totalAmount: "1000",
    milestones: [],
  },
};

/** An id crafted to break out of its URL path segment if left raw. */
const hostileId = "7/../../admin?wipe=true#frag";

describe("lib/grants/api path segment encoding", () => {
  beforeEach(() => {
    fetchMock.mockReset();
    fetchMock.mockImplementation(async (url: string) => {
      if (url.includes("/grant_reviewers/")) {
        return {
          ok: true,
          json: async () => ({ data: [{ reviewerStellarAddress: "GREVIEWER" }] }),
        };
      }
      if (url.endsWith("/funders")) {
        return {
          ok: true,
          json: async () => ({ data: [{ address: "GFUNDER", amount: "42" }] }),
        };
      }
      return { ok: true, json: async () => grantDetail };
    });
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("escapes the id in the grant detail request", async () => {
    await fetchGrantById(hostileId);

    const requested = String(fetchMock.mock.calls[0]?.[0]);
    expect(requested).toBe(`${API_URL}/grants/${encodeURIComponent(hostileId)}`);
    expect(requested).not.toContain("../..");
    expect(requested).not.toContain("wipe=true");
  });

  it("escapes the grant id in the funders request", async () => {
    const rows = await fetchGrantFunders(hostileId);

    const requested = String(fetchMock.mock.calls[0]?.[0]);
    expect(requested).toBe(
      `${API_URL}/grants/${encodeURIComponent(hostileId)}/funders`,
    );
    expect(requested).not.toContain("../..");
    expect(rows).toEqual([{ address: "GFUNDER", amount: 42n }]);
  });

  it("escapes the grant id in the reviewers request", async () => {
    const reviewers = await fetchGrantReviewers(hostileId);

    const requested = String(fetchMock.mock.calls[0]?.[0]);
    expect(requested).toBe(
      `${API_URL}/grant_reviewers/grant/${encodeURIComponent(hostileId)}`,
    );
    expect(requested).not.toContain("../..");
    expect(reviewers).toEqual(["GREVIEWER"]);
  });

  it("encodes every URL issued while loading a grant detail", async () => {
    await fetchGrantById(hostileId);

    const requested = fetchMock.mock.calls.map(([url]) => String(url));
    expect(requested).toEqual([
      `${API_URL}/grants/${encodeURIComponent(hostileId)}`,
      `${API_URL}/grants/${encodeURIComponent(hostileId)}/funders`,
      `${API_URL}/grant_reviewers/grant/${encodeURIComponent(hostileId)}`,
    ]);
  });
});
