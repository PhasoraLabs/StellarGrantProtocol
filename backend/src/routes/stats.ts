import { Router } from "express";
import { Repository } from "typeorm";
import { Grant } from "../entities/Grant";
import { MilestoneProof } from "../entities/MilestoneProof";
import { ResponseCacheService, responseCacheKeys } from "../services/response-cache";

export const buildStatsRouter = (
  grantRepo: Repository<Grant>,
  proofRepo: Repository<MilestoneProof>,
  responseCache: ResponseCacheService,
) => {
  const router = Router();

  router.get("/", async (_req, res, next) => {
    try {
      const cacheKey = responseCacheKeys.stats();
      if (responseCache.isEnabled()) {
        const hit = await responseCache.get(cacheKey);
        if (hit) {
          res.type("application/json").send(hit);
          return;
        }
      }

      const [totalGrants, proofsCount] = await Promise.all([
        grantRepo.count(),
        proofRepo.count(),
      ]);

      const grantsForSum = await grantRepo.find({ select: ["totalAmount"] });
      const totalFunded = grantsForSum.reduce(
        (acc, g) => acc + Number(g.totalAmount || 0),
        0,
      );

      const body = JSON.stringify({
        totalGrants,
        totalFunded,
        milestonesCompleted: proofsCount,
      });

      await responseCache.set(cacheKey, body);
      res.type("application/json").send(body);
    } catch (error) {
      next(error);
    }
  });

  return router;
};
