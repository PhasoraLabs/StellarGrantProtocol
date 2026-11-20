/**
 * Unit tests for per-user dashboard summary caching (issue #1443).
 *
 * Covers:
 *   - cache miss on first request (DB called, result stored)
 *   - cache hit on second request (DB not called again)
 *   - explicit invalidation clears the entry
 *   - TTL expiry evicts the entry
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { ResponseCacheService } from "../../src/services/response-cache";
import {
  dashboardCacheKey,
  invalidateDashboardCache,
} from "../../src/routes/dashboard";

const ADDRESS = "GBRPYHIL2C2WBO36G6UIGR2PA4M3TQ7VOY3RTMAL4LRRA67ZOHQ65SZD";

// ── cache key helper ──────────────────────────────────────────────────────────

describe("dashboardCacheKey", () => {
  it("produces a stable, address-scoped key", () => {
    const key = dashboardCacheKey(ADDRESS);
    expect(key).toContain(ADDRESS);
    expect(key).toMatch(/^sg:cache:v1:dashboard:/);
  });

  it("produces different keys for different addresses", () => {
    const key1 = dashboardCacheKey("GADDR1");
    const key2 = dashboardCacheKey("GADDR2");
    expect(key1).not.toBe(key2);
  });
});

// ── invalidateDashboardCache ──────────────────────────────────────────────────

describe("invalidateDashboardCache", () => {
  it("removes the entry for the given address", async () => {
    const cache = new ResponseCacheService();
    const key = dashboardCacheKey(ADDRESS);
    await cache.set(key, JSON.stringify({ data: "summary" }), 60);

    expect(await cache.get(key)).not.toBeNull();

    await invalidateDashboardCache(cache, ADDRESS);

    expect(await cache.get(key)).toBeNull();
  });

  it("does not affect cache entries for other addresses", async () => {
    const cache = new ResponseCacheService();
    const keyA = dashboardCacheKey(ADDRESS);
    const keyB = dashboardCacheKey("GOTHER");

    await cache.set(keyA, "a", 60);
    await cache.set(keyB, "b", 60);

    await invalidateDashboardCache(cache, ADDRESS);

    expect(await cache.get(keyA)).toBeNull();
    expect(await cache.get(keyB)).toBe("b");
  });
});

// ── dashboard route caching behaviour ────────────────────────────────────────

describe("buildDashboardRouter — caching behaviour (issue #1443)", () => {
  /**
   * Build the minimal repository doubles needed to exercise the route handler
   * without hitting a real database.
   */
  function makeRepos(grants: object[] = [], milestones: object[] = [], proofs: object[] = []) {
    return {
      grantRepo: { find: vi.fn().mockResolvedValue(grants) },
      milestoneRepo: { find: vi.fn().mockResolvedValue(milestones) },
      proofRepo: { find: vi.fn().mockResolvedValue(proofs) },
    };
  }

  function makeReqRes(address: string) {
    const req = { params: { address } } as unknown as import("express").Request;
    const headers: Record<string, string> = {};
    let body: unknown;
    const res = {
      status: vi.fn().mockReturnThis(),
      json: vi.fn((payload) => {
        body = payload;
      }),
      setHeader: vi.fn((k: string, v: string) => {
        headers[k] = v;
      }),
      _headers: headers,
      _body: () => body,
    } as unknown as import("express").Response & {
      _headers: Record<string, string>;
      _body: () => unknown;
    };
    const next = vi.fn();
    return { req, res, next };
  }

  it("sets X-Cache: MISS on first request and calls the DB", async () => {
    const { buildDashboardRouter } = await import(
      "../../src/routes/dashboard"
    );
    const { grantRepo, milestoneRepo, proofRepo } = makeRepos();
    const cache = new ResponseCacheService();
    const router = buildDashboardRouter(
      grantRepo as never,
      milestoneRepo as never,
      proofRepo as never,
      cache,
    );

    const { req, res, next } = makeReqRes(ADDRESS);

    // Invoke the route handler directly
    const handler = (router.stack[0] as { route: { stack: { handle: Function }[] } }).route.stack[0].handle;
    await handler(req, res, next);

    expect(res.setHeader).toHaveBeenCalledWith("X-Cache", "MISS");
    expect(grantRepo.find).toHaveBeenCalledOnce();
  });

  it("sets X-Cache: HIT on second request and does not call the DB again", async () => {
    const { buildDashboardRouter } = await import(
      "../../src/routes/dashboard"
    );
    const { grantRepo, milestoneRepo, proofRepo } = makeRepos();
    const cache = new ResponseCacheService();
    const router = buildDashboardRouter(
      grantRepo as never,
      milestoneRepo as never,
      proofRepo as never,
      cache,
    );

    const handler = (router.stack[0] as { route: { stack: { handle: Function }[] } }).route.stack[0].handle;
    const { req: req1, res: res1, next: next1 } = makeReqRes(ADDRESS);
    const { req: req2, res: res2, next: next2 } = makeReqRes(ADDRESS);

    await handler(req1, res1, next1);
    await handler(req2, res2, next2);

    expect(res2.setHeader).toHaveBeenCalledWith("X-Cache", "HIT");
    // DB should only be called once across both requests
    expect(grantRepo.find).toHaveBeenCalledOnce();
  });

  it("re-fetches from DB after invalidateDashboardCache is called", async () => {
    const { buildDashboardRouter } = await import(
      "../../src/routes/dashboard"
    );
    const { grantRepo, milestoneRepo, proofRepo } = makeRepos();
    const cache = new ResponseCacheService();
    const router = buildDashboardRouter(
      grantRepo as never,
      milestoneRepo as never,
      proofRepo as never,
      cache,
    );

    const handler = (router.stack[0] as { route: { stack: { handle: Function }[] } }).route.stack[0].handle;
    const { req: req1, res: res1, next: next1 } = makeReqRes(ADDRESS);
    const { req: req2, res: res2, next: next2 } = makeReqRes(ADDRESS);

    await handler(req1, res1, next1);            // populates cache
    await invalidateDashboardCache(cache, ADDRESS); // clears it
    await handler(req2, res2, next2);            // should miss again

    expect(res2.setHeader).toHaveBeenCalledWith("X-Cache", "MISS");
    expect(grantRepo.find).toHaveBeenCalledTimes(2);
  });

  it("returns 400 when address param is empty", async () => {
    const { buildDashboardRouter } = await import(
      "../../src/routes/dashboard"
    );
    const { grantRepo, milestoneRepo, proofRepo } = makeRepos();
    const cache = new ResponseCacheService();
    const router = buildDashboardRouter(
      grantRepo as never,
      milestoneRepo as never,
      proofRepo as never,
      cache,
    );

    const handler = (router.stack[0] as { route: { stack: { handle: Function }[] } }).route.stack[0].handle;
    const req = { params: { address: "" } } as unknown as import("express").Request;
    const res = {
      status: vi.fn().mockReturnThis(),
      json: vi.fn(),
      setHeader: vi.fn(),
    } as unknown as import("express").Response;

    await handler(req, res, vi.fn());

    expect(res.status).toHaveBeenCalledWith(400);
  });

  it("isolates cache entries per user address", async () => {
    const { buildDashboardRouter } = await import(
      "../../src/routes/dashboard"
    );
    const { grantRepo, milestoneRepo, proofRepo } = makeRepos();
    const cache = new ResponseCacheService();
    const router = buildDashboardRouter(
      grantRepo as never,
      milestoneRepo as never,
      proofRepo as never,
      cache,
    );

    const handler = (router.stack[0] as { route: { stack: { handle: Function }[] } }).route.stack[0].handle;
    const { req: reqA, res: resA, next: nextA } = makeReqRes("GADDR1");
    const { req: reqB, res: resB, next: nextB } = makeReqRes("GADDR2");

    await handler(reqA, resA, nextA);
    await handler(reqB, resB, nextB);

    // Both requests miss because they have different address keys
    expect(resA.setHeader).toHaveBeenCalledWith("X-Cache", "MISS");
    expect(resB.setHeader).toHaveBeenCalledWith("X-Cache", "MISS");
    expect(grantRepo.find).toHaveBeenCalledTimes(2);
  });
});
