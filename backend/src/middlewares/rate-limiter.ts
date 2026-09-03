import rateLimit, { Options } from "express-rate-limit";
import { Request } from "express";
import { DataSource } from "typeorm";
import { RateLimitLog } from "../entities/RateLimitLog";
import { env } from "../config/env";
import { metricsService } from "../services/metrics-service";

// Lazily initialised Redis store (only when REDIS_URL is set)
let cachedStore: Options["store"] | undefined;
let storeInitStarted = false;

async function buildRedisStore(): Promise<Options["store"] | undefined> {
  if (!env.redisUrl) return undefined;
  if (storeInitStarted) return cachedStore;
  storeInitStarted = true;
  try {
    const { RedisStore } = await import("rate-limit-redis");
    const { default: Redis } = await import("ioredis");
    const redis = new Redis(env.redisUrl, {
      lazyConnect: true,
      maxRetriesPerRequest: 1,
    });
    cachedStore = new RedisStore({
      sendCommand: (...args: string[]) =>
        redis.call(args[0], ...args.slice(1)) as Promise<number>,
    });
  } catch {
    // Redis unavailable — silently fall back to in-memory
  }
  return cachedStore;
}

export function extractWalletAddress(req: Request): string | null {
  const body = req.body as Record<string, unknown> | undefined;
  if (!body) return null;
  return (
    (body.address as string | undefined) ??
    (body.submittedBy as string | undefined) ??
    (body.reviewer as string | undefined) ??
    null
  );
}

export const createRateLimiter = (dataSource: DataSource) => {
  const repo = dataSource.getRepository(RateLimitLog);

  return rateLimit({
    windowMs: 60 * 1000,
    max: 60,
    standardHeaders: true,
    legacyHeaders: false,

    handler: async (req, res) => {
      try {
        await repo.save({
          ip: req.ip,
          path: req.originalUrl,
          method: req.method,
          userAgent: String(req.headers["user-agent"] || ""),
          address:
            typeof req.headers["x-user-address"] === "string"
              ? req.headers["x-user-address"]
              : null,
        });
      } catch {
        // ignore logging failures
      }

      res.status(429).json({ error: "Too many requests" });
    },
  });
};

export function createWalletRateLimiter(
  windowMs: number,
  max: number,
  keyExtractor: (req: Request) => string | null = extractWalletAddress
) {
  let upgradeAttempted = false;

  const limiter = rateLimit({
    windowMs,
    max,
    standardHeaders: true,
    legacyHeaders: false,

    keyGenerator: (req) => {
      const wallet = keyExtractor(req);
      return wallet ? `wallet:${wallet}` : (req.ip ?? "unknown");
    },

    handler: (req, res) => {
      const wallet = keyExtractor(req);
      const endpoint = req.path;
      console.warn("[rate-limiter] wallet limit hit", { wallet, endpoint });
      metricsService.recordWalletRateLimitHit(endpoint);

      res.status(429).json({
        error: "Too many requests from this wallet address",
        retryAfter: Math.ceil(windowMs / 1000),
      });
    },
  });

  return (
    req: Parameters<typeof limiter>[0],
    res: Parameters<typeof limiter>[1],
    next: Parameters<typeof limiter>[2]
  ) => {
    // Upgrade to Redis store once, non-blocking
    if (!upgradeAttempted) {
      upgradeAttempted = true;
      void buildRedisStore().then((store) => {
        if (store) {
          (limiter as unknown as { store: Options["store"] }).store = store;
        }
      });
    }
    return limiter(req, res, next);
  };
}

// Pre-built per-wallet limiters for each write endpoint
export const walletLimiters = {
  createGrant: createWalletRateLimiter(60 * 60 * 1000, 5),
  fundGrant: createWalletRateLimiter(10 * 60 * 1000, 10),
  milestoneProof: createWalletRateLimiter(60 * 60 * 1000, 20, (req) =>
    (req.body as { submittedBy?: string } | undefined)?.submittedBy ??
    extractWalletAddress(req)
  ),
  milestoneApproval: createWalletRateLimiter(10 * 60 * 1000, 50, (req) =>
    (req.body as { reviewer?: string } | undefined)?.reviewer ??
    extractWalletAddress(req)
  ),
  grantFeedback: createWalletRateLimiter(24 * 60 * 60 * 1000, 1),
  grantDraft: createWalletRateLimiter(60 * 60 * 1000, 20),
  disputeArgument: createWalletRateLimiter(24 * 60 * 60 * 1000, 3),
};
