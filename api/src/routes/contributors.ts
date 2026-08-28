import { Router } from "express";
import { Repository, DataSource, In } from "typeorm";
import { z } from "zod";
import { StrKey } from "@stellar/stellar-sdk";
import { Contributor } from "../entities/Contributor";
import { Grant } from "../entities/Grant";
import { MilestoneProof } from "../entities/MilestoneProof";
import { Milestone } from "../entities/Milestone";
import { Activity } from "../entities/Activity";
import { validateParams, validateQuery } from "../middlewares/validation-middleware";
import { stellarAddressSchema, paginationSchema } from "../schemas";

const contributorsQuerySchema = paginationSchema.extend({
  q: z.string().optional(),
  sort: z.enum(["reputation", "grants", "earned"]).default("reputation"),
  order: z.enum(["ASC", "DESC"]).default("DESC"),
});

const addressParamSchema = z.object({
  address: z.string().regex(/^G[A-Z2-7]{55}$/),
});

const contributorGrantsQuerySchema = paginationSchema.extend({
  sort: z.enum(["id", "updatedAt", "totalAmount"]).default("id"),
  order: z.enum(["ASC", "DESC"]).default("ASC"),
});

export const buildContributorsRouter = (
  contributorRepo: Repository<Contributor>,
  grantRepo: Repository<Grant>,
  milestoneProofRepo: Repository<MilestoneProof>,
  activityRepo: Repository<Activity>,
) => {
  const router = Router();

  // GET /contributors - paginated contributor list with optional search
  router.get("/", validateQuery(contributorsQuerySchema), async (req, res, next) => {
    try {
      const { page, limit, q, sort, order } = (req as any).validatedQuery;

      const queryBuilder = contributorRepo.createQueryBuilder("c");

      if (q) {
        queryBuilder.andWhere("c.address LIKE :q", { q: `${q}%` });
      }

      // Apply sorting
      switch (sort) {
        case "reputation":
          queryBuilder.orderBy("c.reputation", order);
          break;
        case "grants":
          queryBuilder.orderBy("c.totalGrantsCompleted", order);
          break;
        case "earned":
          // Note: totalEarned is not in Contributor entity, using reputation as fallback
          queryBuilder.orderBy("c.reputation", order);
          break;
      }

      queryBuilder.skip((page - 1) * limit).take(limit);

      const [contributors, total] = await queryBuilder.getManyAndCount();

      res.json({
        data: contributors.map((c) => ({
          address: c.address,
          reputation: c.reputation,
          totalGrantsCompleted: c.totalGrantsCompleted,
        })),
        meta: {
          total,
          page,
          limit,
          totalPages: Math.ceil(total / limit),
        },
      });
    } catch (error) {
      next(error);
    }
  });

  // GET /contributors/:address - full contributor profile
  router.get("/:address", validateParams(addressParamSchema), async (req, res, next) => {
    try {
      const { address } = (req as any).validatedParams;

      const contributor = await contributorRepo.findOne({ where: { address } });
      if (!contributor) {
        res.status(404).json({ error: "Contributor not found" });
        return;
      }

      // Get grants where this address was recipient
      const recipientGrants = await grantRepo.find({
        where: { recipient: address, isDraft: false },
        order: { id: "DESC" },
        take: 5,
      });
      const recipientGrantIds = recipientGrants.map((g) => g.id);

      // Calculate approval rate
      let approvalRate = 0;
      if (recipientGrantIds.length > 0) {
        const totalSubmitted = await milestoneProofRepo.count({
          where: { grantId: In(recipientGrantIds) },
        });
        approvalRate = totalSubmitted > 0 ? 1.0 : 0; // Simplification: all paid milestones were approved
      }

      // Get memberSince from earliest Activity
      const earliestActivity = await activityRepo.findOne({
        where: { actorAddress: address },
        order: { timestamp: "ASC" },
      });
      const memberSince = earliestActivity?.timestamp?.toISOString() || null;

      // Extract skills from bio or github (simplified)
      const skills: string[] = [];
      if (contributor.bio) {
        // Simple extraction of common tech keywords
        const techKeywords = ["rust", "stellar", "defi", "blockchain", "javascript", "typescript", "python", "solidity"];
        techKeywords.forEach((keyword) => {
          if (contributor.bio?.toLowerCase().includes(keyword)) {
            skills.push(keyword);
          }
        });
      }

      // Extract github handle from githubUrl
      let githubHandle = null;
      if (contributor.githubUrl) {
        const match = contributor.githubUrl.match(/github\.com\/([^\/]+)/);
        if (match) {
          githubHandle = match[1];
        }
      }

      // Get recent grants
      const recentGrants = recipientGrants.slice(0, 5).map((g) => ({
        id: g.id,
        title: g.title,
        status: g.status,
        totalAmount: g.totalAmount,
      }));

      res.json({
        data: {
          address: contributor.address,
          reputationScore: contributor.reputation,
          grantsCompleted: contributor.totalGrantsCompleted,
          totalEarned: "0", // Not tracked in current schema
          approvalRate,
          memberSince,
          skills,
          bio: contributor.bio,
          githubHandle,
          recentGrants,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  // GET /contributors/:address/grants - grants where this address was recipient
  router.get(
    "/:address/grants",
    validateParams(addressParamSchema),
    validateQuery(contributorGrantsQuerySchema),
    async (req, res, next) => {
      try {
        const { address } = (req as any).validatedParams;
        const { page, limit, sort, order } = (req as any).validatedQuery;

        const queryBuilder = grantRepo
          .createQueryBuilder("g")
          .where("g.recipient = :address", { address })
          .andWhere("g.isDraft = :isDraft", { isDraft: false });

        // Apply sorting
        switch (sort) {
          case "id":
            queryBuilder.orderBy("g.id", order);
            break;
          case "updatedAt":
            queryBuilder.orderBy("g.updatedAt", order);
            break;
          case "totalAmount":
            queryBuilder.orderBy("g.totalAmount", order);
            break;
        }

        queryBuilder.skip((page - 1) * limit).take(limit);

        const [grants, total] = await queryBuilder.getManyAndCount();

        // Get milestone summary for each grant
        const grantIds = grants.map((g) => g.id);
        const milestoneCounts = await milestoneProofRepo
          .createQueryBuilder("mp")
          .select("mp.grantId", "grantId")
          .addSelect("COUNT(*)", "count")
          .where("mp.grantId IN (:...grantIds)", { grantIds })
          .groupBy("mp.grantId")
          .getRawMany();

        const milestoneCountMap = new Map(
          milestoneCounts.map((mc) => [mc.grantId, parseInt(mc.count, 10)]),
        );

        const grantsWithSummary = grants.map((g) => ({
          id: g.id,
          title: g.title,
          description: g.description,
          status: g.status,
          totalAmount: g.totalAmount,
          recipient: g.recipient,
          milestoneCount: milestoneCountMap.get(g.id) || 0,
          updatedAt: g.updatedAt,
        }));

        res.json({
          data: grantsWithSummary,
          meta: {
            total,
            page,
            limit,
            totalPages: Math.ceil(total / limit),
          },
        });
      } catch (error) {
        next(error);
      }
    },
  );

  // GET /contributors/:address/milestones - approved/paid milestones for this address
  router.get(
    "/:address/milestones",
    validateParams(addressParamSchema),
    validateQuery(paginationSchema),
    async (req, res, next) => {
      try {
        const { address } = (req as any).validatedParams;
        const { page, limit } = (req as any).validatedQuery;

        // Get grants where this address was recipient
        const recipientGrants = await grantRepo.find({
          where: { recipient: address, isDraft: false },
          select: ["id"],
        });
        const recipientGrantIds = recipientGrants.map((g) => g.id);

        if (recipientGrantIds.length === 0) {
          res.json({
            data: [],
            meta: {
              total: 0,
              page,
              limit,
              totalPages: 0,
            },
          });
          return;
        }

        // Get MilestoneProof records for this address, joined with Grant and Milestone data
        const queryBuilder = milestoneProofRepo
          .createQueryBuilder("mp")
          .leftJoin(Grant, "g", "g.id = mp.grantId")
          .leftJoin(Milestone, "m", "m.grantId = mp.grantId AND m.idx = mp.milestoneIdx")
          .select([
            "mp.id",
            "mp.grantId",
            "mp.milestoneIdx",
            "mp.proofCid",
            "mp.description",
            "mp.submittedBy",
            "mp.createdAt",
            "g.id",
            "g.title",
            "g.recipient",
            "m.title",
            "m.description",
          ])
          .where("mp.grantId IN (:...grantIds)", { grantIds: recipientGrantIds })
          .andWhere("mp.submittedBy = :address", { address })
          .orderBy("mp.createdAt", "DESC")
          .skip((page - 1) * limit)
          .take(limit);

        const [milestones, total] = await queryBuilder.getManyAndCount();

        const milestonesWithDetails = milestones.map((mp) => ({
          id: mp.id,
          grantId: mp.grantId,
          milestoneIdx: mp.milestoneIdx,
          proofCid: mp.proofCid,
          description: mp.description,
          submittedBy: mp.submittedBy,
          createdAt: mp.createdAt,
          grant: {
            id: (mp as any).g_id,
            title: (mp as any).g_title,
            recipient: (mp as any).g_recipient,
          },
          milestone: {
            title: (mp as any).m_title,
            description: (mp as any).m_description,
          },
        }));

        res.json({
          data: milestonesWithDetails,
          meta: {
            total,
            page,
            limit,
            totalPages: Math.ceil(total / limit),
          },
        });
      } catch (error) {
        next(error);
      }
    },
  );

  return router;
};
