import { Router } from "express";
import { DataSource, Like } from "typeorm";
import { Grant } from "../entities/Grant";
import { Contributor } from "../entities/Contributor";
import { MilestoneProof } from "../entities/MilestoneProof";
import { validateQuery } from "../middlewares/validation-middleware";
import { searchQuerySchema } from "../schemas";
import { createWalletRateLimiter } from "../middlewares/rate-limiter";

/**
 * Unified Search Endpoint
 *
 * Searches across:
 * 1. Grants (title, tags)
 * 2. Contributors (address only — email is PII and must not be exposed)
 * 3. Milestone Descriptions (description, CID)
 */

// 30 searches per minute per IP/wallet to prevent de-anonymisation probing
// and unindexed LIKE-scan amplification.
const searchRateLimiter = createWalletRateLimiter(60 * 1000, 30);

export const buildSearchRouter = (dataSource: DataSource) => {
  const router = Router();

  router.get(
    "/",
    searchRateLimiter,
    validateQuery(searchQuerySchema),
    async (req, res, next) => {
      try {
        const { q } = (req as any).validatedQuery;
        const query = q.trim();

        if (!query || query.length < 2) {
          return res.json({ data: [] });
        }

        const searchPattern = `%${query}%`;

        const [grants, contributors, milestones] = await Promise.all([
          dataSource.getRepository(Grant).find({
            where: [
              { title: Like(searchPattern) },
              { tags: Like(searchPattern) },
            ],
            take: 50,
          }),
          // Only search public-facing fields — email is PII and must not be
          // matched or returned to unauthenticated callers.
          dataSource.getRepository(Contributor).find({
            where: [{ address: Like(searchPattern) }],
            take: 50,
          }),
          dataSource.getRepository(MilestoneProof).find({
            where: [
              { description: Like(searchPattern) },
              { proofCid: Like(searchPattern) },
            ],
            take: 50,
          }),
        ]);

        const results = [
          ...grants.map((g) => ({ id: g.id, name: g.title, type: "grant" })),
          ...contributors.map((c) => ({
            id: c.address,
            name: c.address,
            type: "contributor",
          })),
          ...milestones.map((m) => ({
            id: m.id,
            name: m.description || m.proofCid,
            type: "milestone",
          })),
        ];

        res.json({ data: results.slice(0, 50) });
      } catch (error) {
        next(error);
      }
    },
  );

  return router;
};
