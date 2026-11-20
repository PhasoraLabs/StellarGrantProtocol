import { Router } from "express";
import { In, Repository } from "typeorm";
import { Grant } from "../entities/Grant";
import { Milestone } from "../entities/Milestone";
import { MilestoneProof } from "../entities/MilestoneProof";
import { createProofLookup, enrichMilestone } from "../utils/milestones";
import { ResponseCacheService } from "../services/response-cache";

/** Cache TTL for dashboard summaries (60 seconds). */
const DASHBOARD_CACHE_TTL_S = 60;

/** Cache key for a user's dashboard summary, keyed by their Stellar address. */
export const dashboardCacheKey = (address: string) =>
  `sg:cache:v1:dashboard:${address}`;

/**
 * Invalidate the cached dashboard summary for one address.
 * Call this whenever a grant or milestone for that address changes.
 */
export async function invalidateDashboardCache(
  cache: ResponseCacheService,
  address: string,
): Promise<void> {
  await cache.delete(dashboardCacheKey(address));
}

/**
 * Dashboard route — reads already-synced data from the database.
 *
 * The previous implementation called syncService.syncAllGrants() on every
 * request, turning each GET into a full on-chain resync.  That is a DoS
 * amplification vector: a single unauthenticated request triggers N Soroban
 * RPC calls.  Sync now runs only on its own scheduled interval or via the
 * admin-triggered endpoint; the dashboard simply reads what is already stored.
 *
 * Per-user summary caching (issue #1443) avoids re-running the DB aggregation
 * on every request.  The cache entry expires after DASHBOARD_CACHE_TTL_S
 * seconds and is explicitly invalidated via invalidateDashboardCache() when
 * a relevant event (grant or milestone change) touches that user.
 */
export const buildDashboardRouter = (
  grantRepo: Repository<Grant>,
  milestoneRepo: Repository<Milestone>,
  proofRepo: Repository<MilestoneProof>,
  cache: ResponseCacheService = new ResponseCacheService(),
) => {
  const router = Router();

  router.get("/:address", async (req, res, next) => {
    try {
      const address = String(req.params.address || "").trim();
      if (!address) {
        res.status(400).json({ error: "Address is required" });
        return;
      }

      // ── Cache hit ──────────────────────────────────────────────────────
      const cacheKey = dashboardCacheKey(address);
      const cached = await cache.get(cacheKey);
      if (cached !== null) {
        res.setHeader("X-Cache", "HIT");
        res.json(JSON.parse(cached));
        return;
      }

      // ── Compute summary ────────────────────────────────────────────────
      const grants = await grantRepo.find({
        where: { recipient: address },
        order: { updatedAt: "DESC" },
      });

      if (grants.length === 0) {
        const payload = {
          data: {
            address,
            summary: { upcomingCount: 0, overdueCount: 0 },
            upcomingDeadlines: [],
            overdueMilestones: [],
          },
        };
        await cache.set(cacheKey, JSON.stringify(payload), DASHBOARD_CACHE_TTL_S);
        res.setHeader("X-Cache", "MISS");
        res.json(payload);
        return;
      }

      const grantIds = grants.map((grant) => grant.id);
      const milestones = await milestoneRepo.find({
        where: { grantId: In(grantIds) },
        order: { deadline: "ASC", idx: "ASC" },
      });
      const proofs = await proofRepo.find({
        where: { grantId: In(grantIds) },
        select: {
          grantId: true,
          milestoneIdx: true,
          createdAt: true,
        },
      });

      const proofLookup = createProofLookup(proofs);
      const grantMap = new Map(grants.map((grant) => [grant.id, grant]));
      const enriched = milestones.map((milestone) => {
        const base = enrichMilestone(
          milestone,
          proofLookup.get(`${milestone.grantId}:${milestone.idx}`),
        );
        return {
          ...base,
          grantTitle:
            grantMap.get(milestone.grantId)?.title ??
            `Grant #${milestone.grantId}`,
          grantStatus: grantMap.get(milestone.grantId)?.status ?? "unknown",
        };
      });

      const upcomingDeadlines = enriched.filter(
        (milestone) => !milestone.submitted && !milestone.overdue,
      );
      const overdueMilestones = enriched.filter(
        (milestone) => milestone.overdue,
      );

      const payload = {
        data: {
          address,
          summary: {
            upcomingCount: upcomingDeadlines.length,
            overdueCount: overdueMilestones.length,
          },
          upcomingDeadlines,
          overdueMilestones,
        },
      };

      // ── Store in cache ─────────────────────────────────────────────────
      await cache.set(cacheKey, JSON.stringify(payload), DASHBOARD_CACHE_TTL_S);
      res.setHeader("X-Cache", "MISS");
      res.json(payload);
    } catch (error) {
      next(error);
    }
  });

  return router;
};
