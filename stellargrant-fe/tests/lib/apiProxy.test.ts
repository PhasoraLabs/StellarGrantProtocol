import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { proxyGet } from "@/lib/apiProxy";

const fetchMock = vi.fn();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

function makeRequest(url: string): Request {
  return new Request(url);
}

describe("proxyGet", () => {
  it("forwards the upstream payload on success with the right cache header", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ grants: ["a", "b"] }), { status: 200 }),
    );

    const res = await proxyGet(
      makeRequest("http://test/api/grants?cursor=1&limit=10"),
      "/grants",
      { revalidate: 30 },
    );

    const upstreamUrl = fetchMock.mock.calls[0][0] as string;
    expect(upstreamUrl).toContain("/grants?cursor=1&limit=10");
    expect(res.status).toBe(200);
    expect(res.headers.get("cache-control")).toBe(
      "public, s-maxage=30, stale-while-revalidate=30",
    );
    expect(await res.json()).toEqual({ grants: ["a", "b"] });
  });

  it("normalises an upstream non-ok response", async () => {
    fetchMock.mockResolvedValueOnce(new Response("nope", { status: 502 }));
    const res = await proxyGet(makeRequest("http://test/api/x"), "/x", {
      revalidate: 5,
    });
    expect(res.status).toBe(502);
    expect(await res.json()).toEqual({
      error: "Upstream returned 502",
      code: "UPSTREAM_ERROR",
      status: 502,
    });
  });

  it("normalises a network failure into a 503", async () => {
    fetchMock.mockRejectedValueOnce(new TypeError("ECONNREFUSED"));
    const res = await proxyGet(makeRequest("http://test/api/x"), "/x", {
      revalidate: 5,
    });
    expect(res.status).toBe(503);
    expect(await res.json()).toEqual({
      error: "Failed to reach upstream API",
      code: "UPSTREAM_UNREACHABLE",
      status: 503,
    });
  });

  it("normalises an aborted upstream into a timeout error", async () => {
    fetchMock.mockImplementationOnce(() => {
      const e = new Error("aborted");
      e.name = "AbortError";
      return Promise.reject(e);
    });
    const res = await proxyGet(makeRequest("http://test/api/x"), "/x", {
      revalidate: 5,
    });
    expect(res.status).toBe(503);
    expect(await res.json()).toEqual({
      error: "Upstream request timed out",
      code: "UPSTREAM_TIMEOUT",
      status: 503,
    });
  });

  it("can skip query forwarding when forwardSearch is false", async () => {
    fetchMock.mockResolvedValueOnce(new Response("{}", { status: 200 }));
    await proxyGet(
      makeRequest("http://test/api/stats?cursor=xyz"),
      "/stats",
      { revalidate: 60, forwardSearch: false },
    );
    const upstreamUrl = fetchMock.mock.calls[0][0] as string;
    expect(upstreamUrl).not.toContain("?");
  });
});
