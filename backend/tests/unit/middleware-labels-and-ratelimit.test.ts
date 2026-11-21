/**
 * Unit tests covering:
 *   #1438 — metrics-middleware captures route template and status code labels
 *   #1442 — rate-limiter keys by authenticated user ID when present
 */

import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { metricsService } from "../../src/services/metrics-service";
import { metricsMiddleware } from "../../src/middlewares/metrics-middleware";

// ── #1438: metrics-middleware ─────────────────────────────────────────────────

describe("metricsMiddleware — route and status code labels (issue #1438)", () => {
  it("calls observeHttpRequest with the route template once the response finishes", () => {
    // Arrange: build a minimal fake req/res pair
    const observeSpy = vi.spyOn(metricsService, "observeHttpRequest").mockImplementation(() => {});

    const finishListeners: Array<() => void> = [];
    const req = {
      method: "GET",
      path: "/grants/123",
      baseUrl: "/api",
      route: { path: "/grants/:id" },
    } as unknown as import("express").Request;

    const res = {
      statusCode: 200,
      on: (event: string, cb: () => void) => {
        if (event === "finish") finishListeners.push(cb);
      },
    } as unknown as import("express").Response;

    const next = vi.fn();

    // Act
    metricsMiddleware(req, res, next);
    expect(next).toHaveBeenCalled();

    // Trigger finish
    finishListeners.forEach((fn) => fn());

    // Assert: route label uses the template, not the raw path
    expect(observeSpy).toHaveBeenCalledOnce();
    const [method, route, status] = observeSpy.mock.calls[0] as [
      string,
      string,
      number,
      number,
    ];
    expect(method).toBe("GET");
    expect(route).toBe("/api/grants/:id");
    expect(status).toBe(200);

    observeSpy.mockRestore();
  });

  it("falls back to req.path when no route is matched", () => {
    const observeSpy = vi.spyOn(metricsService, "observeHttpRequest").mockImplementation(() => {});

    const finishListeners: Array<() => void> = [];
    const req = {
      method: "POST",
      path: "/unknown-path",
      baseUrl: "",
      route: undefined,
    } as unknown as import("express").Request;

    const res = {
      statusCode: 404,
      on: (event: string, cb: () => void) => {
        if (event === "finish") finishListeners.push(cb);
      },
    } as unknown as import("express").Response;

    metricsMiddleware(req, res, vi.fn());
    finishListeners.forEach((fn) => fn());

    const [, route, status] = observeSpy.mock.calls[0] as [
      string,
      string,
      number,
      number,
    ];
    expect(route).toBe("/unknown-path");
    expect(status).toBe(404);

    observeSpy.mockRestore();
  });
});

// ── #1442: rate-limiter user-ID keying ────────────────────────────────────────

describe("extractUserId — per-user rate limit keying (issue #1442)", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns the user id as a string when req.user.id is set", async () => {
    const { extractUserId } = await import(
      "../../src/middlewares/rate-limiter"
    );

    const req = { user: { id: 42 } } as unknown as import("express").Request;
    expect(extractUserId(req)).toBe("42");
  });

  it("returns the user id when it is already a string", async () => {
    const { extractUserId } = await import(
      "../../src/middlewares/rate-limiter"
    );

    const req = {
      user: { id: "user-abc" },
    } as unknown as import("express").Request;
    expect(extractUserId(req)).toBe("user-abc");
  });

  it("returns undefined when there is no authenticated user", async () => {
    const { extractUserId } = await import(
      "../../src/middlewares/rate-limiter"
    );

    const req = {} as import("express").Request;
    expect(extractUserId(req)).toBeUndefined();
  });

  it("returns undefined when req.user exists but id is falsy", async () => {
    const { extractUserId } = await import(
      "../../src/middlewares/rate-limiter"
    );

    const req = { user: {} } as unknown as import("express").Request;
    expect(extractUserId(req)).toBeUndefined();
  });
});
