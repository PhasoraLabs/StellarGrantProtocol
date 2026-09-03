import { Router } from "express";
import { In, Repository } from "typeorm";
import { Grant } from "../entities/Grant";
import { Milestone } from "../entities/Milestone";
import { MilestoneProof } from "../entities/MilestoneProof";
import { GrantSyncService } from "../services/grant-sync-service";
import { createProofLookup, enrichMilestone } from "../utils/milestones";

export const buildDashboardRouter = (
  grantRepo: Repository<Grant>,
  milestoneRepo: Repository<Milestone>,
  proofRepo: Repository<MilestoneProof>,
  syncService: GrantSyncService,
) => {
  const router = Router();

  router.get("/:address", async (req, res, next) => {
    try {
      await syncService.syncAllGrants();
      const address = String(req.params.address || "").trim();
      if (!address) {
        res.status(400).json({ error: "Address is required" });
        return;
      }

      const grants = await grantRepo.find({
        where: { recipient: address },
        order: { updatedAt: "DESC" },
      });

      if (grants.length === 0) {
        res.json({
          data: {
            address,
            summary: { upcomingCount: 0, overdueCount: 0 },
            upcomingDeadlines: [],
            overdueMilestones: [],
          },
        });
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
        const base = enrichMilestone(milestone, proofLookup.get(`${milestone.grantId}:${milestone.idx}`));
        return {
          ...base,
          grantTitle: grantMap.get(milestone.grantId)?.title ?? `Grant #${milestone.grantId}`,
          grantStatus: grantMap.get(milestone.grantId)?.status ?? "unknown",
        };
      });

      const upcomingDeadlines = enriched.filter((milestone) => !milestone.submitted && !milestone.overdue);
      const overdueMilestones = enriched.filter((milestone) => milestone.overdue);

      res.json({
        data: {
          address,
          summary: {
            upcomingCount: upcomingDeadlines.length,
            overdueCount: overdueMilestones.length,
          },
          upcomingDeadlines,
          overdueMilestones,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
