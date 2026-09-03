import { Router } from "express";
import { DataSource } from "typeorm";
import Redis from "ioredis";
import { SorobanContractClient } from "../soroban/types";
import { env } from "../config/env";

export const buildHealthRouter = (
  dataSource: DataSource,
  sorobanClient: SorobanContractClient,
): Router => {
  const router = Router();

  // Liveness: just confirm the process is alive
  router.get("/liveness", (_req, res) => {
    res.status(200).json({ status: "ok" });
  });

  // Readiness: verify all dependencies are reachable
  router.get("/readiness", async (_req, res) => {
    const checks: Record<string, "ok" | "fail"> = {};
    let healthy = true;

    // Database
    try {
      await dataSource.query("SELECT 1");
      checks.database = "ok";
    } catch {
      checks.database = "fail";
      healthy = false;
    }

    // Redis (optional — skip check if not configured)
    if (env.redisUrl.trim()) {
      const redis = new Redis(env.redisUrl, { maxRetriesPerRequest: 1, connectTimeout: 2000, lazyConnect: true });
      try {
        await redis.connect();
        await redis.ping();
        checks.redis = "ok";
      } catch {
        checks.redis = "fail";
        healthy = false;
      } finally {
        redis.disconnect();
      }
    }

    // Soroban RPC
    try {
      await sorobanClient.getLatestLedger();
      checks.soroban = "ok";
    } catch {
      checks.soroban = "fail";
      healthy = false;
    }

    res.status(healthy ? 200 : 503).json({ status: healthy ? "ok" : "degraded", checks });
  });

  return router;
};
