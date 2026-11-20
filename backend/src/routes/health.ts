import { Request, Response, Router } from "express";
import { DataSource } from "typeorm";
import Redis from "ioredis";
import { SorobanContractClient } from "../soroban/types";
import { env } from "../config/env";

const SOROBAN_RPC_TIMEOUT_MS = 5_000;

export function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  let timeout: ReturnType<typeof setTimeout> | undefined;
  const timeoutPromise = new Promise<never>((_resolve, reject) => {
    timeout = setTimeout(
      () => reject(new Error(`Operation timed out after ${timeoutMs} ms`)),
      timeoutMs,
    );
  });

  return Promise.race([promise, timeoutPromise]).finally(() => {
    if (timeout) clearTimeout(timeout);
  });
}

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
  const readinessHandler = async (_req: Request, res: Response) => {
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
      await withTimeout(
        sorobanClient.getLatestLedger(),
        SOROBAN_RPC_TIMEOUT_MS,
      );
      checks.soroban = "ok";
    } catch {
      checks.soroban = "fail";
      healthy = false;
    }

    res.status(healthy ? 200 : 503).json({ status: healthy ? "ok" : "degraded", checks });
  };

  router.get("/", readinessHandler);
  router.get("/readiness", readinessHandler);

  return router;
};
