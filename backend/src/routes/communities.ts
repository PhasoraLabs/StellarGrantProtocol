import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { Community } from "../entities/Community";
import { Grant } from "../entities/Grant";
import { env } from "../config/env";
import { Activity } from "../entities/Activity";
import {
  validateBody,
  validateParams,
  validateQuery,
  validateRequest,
} from "../middlewares/validation-middleware";
import {
  communityCreateSchema,
  communityUpdateSchema,
  idParamSchema,
  paginationSchema,
} from "../schemas";
import {
  createRbacMiddleware,
  AuthenticatedRequest,
} from "../middlewares/rbac-middleware";
import { RbacService } from "../services/rbac-service";
import { Permission } from "../config/rbac";
import { WebhookDispatcher } from "../services/webhook-dispatcher";
import { WebhookEventType } from "../entities/WebhookSubscription";

const isPlatformAdmin = (address?: string) =>
  !!address && env.adminAddresses.includes(address);

export const buildCommunitiesRouter = (
  communityRepo: Repository<Community>,
  grantRepo: Repository<Grant>,
  activityRepo: Repository<Activity>,
  rbacService: RbacService,
  webhookDispatcher?: WebhookDispatcher,
) => {
  const router = Router();
  const { requirePermission, requireAnyPermission } =
    createRbacMiddleware(rbacService);

  router.get("/", async (_req, res, next) => {
    try {
      const communities = await communityRepo.find({
        order: { featured: "DESC", name: "ASC" },
      });
      res.json({ data: communities });
    } catch (error) {
      next(error);
    }
  });

  router.post(
    "/",
    requirePermission("communities:create" as Permission),
    validateBody(communityCreateSchema),
    async (req: AuthenticatedRequest, res, next) => {
      try {
        const payload = (req as any).validatedBody;
        // Only use verified session identity - remove header fallback
        const stellarAddress = req.user?.stellarAddress;

        if (!stellarAddress) {
          res.status(401).json({ error: "Authentication required" });
          return;
        }

        const created = await communityRepo.save({
          name: payload.name.trim(),
          description: payload.description?.trim() ?? null,
          logoUrl: payload.logoUrl ?? null,
          adminAddresses: payload.adminAddresses?.map((address: string) =>
            address.trim(),
          ) ?? [stellarAddress],
          featured: payload.featured ?? false,
        });

        webhookDispatcher?.dispatch(WebhookEventType.COMMUNITY_CREATED, {
          communityId: created.id,
          name: created.name,
          adminAddresses: created.adminAddresses,
        });

        res.status(201).json({ data: created });
      } catch (error: any) {
        if (error?.code === "23505" || error?.code === "SQLITE_CONSTRAINT") {
          res.status(409).json({ error: "Community name already exists" });
          return;
        }
        next(error);
      }
    },
  );

  router.patch(
    "/:id",
    requireAnyPermission(["communities:update", "admin:all"] as Permission[]),
    validateRequest({ params: idParamSchema, body: communityUpdateSchema }),
    async (req: AuthenticatedRequest, res, next) => {
      try {
        const { id } = (req as any).validatedParams;
        // Only use verified session identity - remove header fallback
        const stellarAddress = req.user?.stellarAddress;

        if (!stellarAddress) {
          res.status(401).json({ error: "Authentication required" });
          return;
        }

        const community = await communityRepo.findOne({ where: { id } });
        if (!community) {
          res.status(404).json({ error: "Community not found" });
          return;
        }

        const canManage =
          isPlatformAdmin(stellarAddress) ||
          (community.adminAddresses ?? []).includes(stellarAddress);
        if (!canManage) {
          res.status(403).json({ error: "Community admin privileges required" });
          return;
        }

        const payload = (req as any).validatedBody;
        if (payload.description !== undefined) {
          community.description = payload.description.trim();
        }
        if (payload.logoUrl !== undefined) {
          community.logoUrl = payload.logoUrl;
        }
        if (payload.featured !== undefined) {
          community.featured = payload.featured;
        }

        const saved = await communityRepo.save(community);

        webhookDispatcher?.dispatch(WebhookEventType.COMMUNITY_UPDATED, {
          communityId: saved.id,
          name: saved.name,
          featured: saved.featured,
        });

        res.json({ data: saved });
      } catch (error) {
        next(error);
      }
    },
  );

  router.get(
    "/:id/grants",
    validateParams(idParamSchema),
    validateQuery(paginationSchema),
    async (req, res, next) => {
      try {
        const { id } = (req as any).validatedParams;
        const { page, limit } = (req as any).validatedQuery;

        const community = await communityRepo.findOne({ where: { id } });
        if (!community) {
          res.status(404).json({ error: "Community not found" });
          return;
        }

        const [grants, total] = await grantRepo.findAndCount({
          where: { communityId: id },
          order: { updatedAt: "DESC" },
          skip: (page - 1) * limit,
          take: limit,
        });

        const grantIds = grants.map((grant) => grant.id);
        const activity = grantIds.length
          ? await activityRepo.find({
              where: grantIds.map((grantId) => ({
                entityType: "grant",
                entityId: grantId,
              })),
              order: { timestamp: "DESC" },
              take: 100,
            })
          : [];

        res.json({
          data: grants,
          community,
          activity,
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

  router.post(
    "/:id/grants/:grantId",
    requirePermission("communities:manage" as Permission),
    validateRequest({
      params: z.object({
        id: z.coerce.number().int().positive(),
        grantId: z.coerce.number().int().positive(),
      }),
    }),
    async (req: AuthenticatedRequest, res, next) => {
      try {
        const { id, grantId } = (req as any).validatedParams;

        const community = await communityRepo.findOne({ where: { id } });
        if (!community) {
          res.status(404).json({ error: "Community not found" });
          return;
        }

        const grant = await grantRepo.findOne({ where: { id: grantId } });
        if (!grant) {
          res.status(404).json({ error: "Grant not found" });
          return;
        }

        grant.communityId = id;
        await grantRepo.save(grant);
        res.json({ data: grant });
      } catch (error) {
        next(error);
      }
    },
  );

  return router;
};
