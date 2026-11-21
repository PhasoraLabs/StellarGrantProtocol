import express from "express";
import request from "supertest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createWalletRateLimiter } from "../../src/middlewares/rate-limiter";

vi.mock("../../src/config/env", () => ({ env: { redisUrl: "" } }));
vi.mock("../../src/services/metrics-service", () => ({
  metricsService: { recordWalletRateLimitHit: vi.fn() },
}));

const createApp = (windowMs: number, max: number) => {
  const app = express();
  app.use(createWalletRateLimiter(windowMs, max, () => "test-wallet"));
  app.get("/", (_req, res) => res.sendStatus(200));
  return app;
};

describe("createWalletRateLimiter window behavior", () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("allows a burst up to the configured limit and rejects the next request", async () => {
    vi.useFakeTimers();
    const app = createApp(1_000, 2);

    expect((await request(app).get("/")).status).toBe(200);
    expect((await request(app).get("/")).status).toBe(200);
    expect((await request(app).get("/")).status).toBe(429);
  });

  it("resets the request count after the configured window", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-01-01T00:00:00.000Z"));
    const app = createApp(1_000, 1);

    expect((await request(app).get("/")).status).toBe(200);
    expect((await request(app).get("/")).status).toBe(429);

    await vi.advanceTimersByTimeAsync(1_001);

    expect((await request(app).get("/")).status).toBe(200);
  });
});
