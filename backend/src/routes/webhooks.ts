import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { User } from "../entities/User";
import { WebhookSubscription } from "../entities/WebhookSubscription";
import { WebhookDispatcher, WebhookPayload } from "../services/webhook-dispatcher";
import { validateBody, validateParams, validateRequest } from "../middlewares/validation-middleware";
import {
  webhookSubscriptionCreateSchema,
  webhookSubscriptionUpdateSchema,
  webhookTestSchema,
  idParamSchema,
} from "../schemas";
import { createRbacMiddleware, AuthenticatedRequest } from "../middlewares/rbac-middleware";
import { RbacService } from "../services/rbac-service";
import { Permission } from "../config/rbac";

export const buildWebhooksRouter = (
  userRepo: Repository<User>,
  subscriptionRepo: Repository<WebhookSubscription>,
  webhookDispatcher: WebhookDispatcher,
  rbacService: RbacService,
) => {
  const router = Router();
  const { requirePermission } = createRbacMiddleware(rbacService);

  /**
   * Helper to get or create user by address
   */
  async function getOrCreateUser(stellarAddress: string): Promise<User> {
    let user = await userRepo.findOne({ where: { stellarAddress } });
    if (!user) {
      user = userRepo.create({
        stellarAddress,
        email: `${stellarAddress}@placeholder.stellar`,
        notifyMilestoneApproved: true,
        notifyMilestoneSubmitted: true,
      });
      await userRepo.save(user);
    }
    return user;
  }

  /**
   * Helper to resolve the caller's stellar address
   */
  function getCallerAddress(req: AuthenticatedRequest): string {
    const address = req.user?.stellarAddress || req.header("x-user-address") || req.header("x-admin-address");
    if (!address) {
      throw new Error("Authentication required");
    }
    return address;
  }

  // ---------------------------------------------------------------------------
  // CRUD endpoints
  // ---------------------------------------------------------------------------

  /**
   * GET /webhooks
   * List subscriptions for the authenticated user
   */
  router.get("/", async (req: AuthenticatedRequest, res, next) => {
    try {
      const address = getCallerAddress(req);
      const user = await getOrCreateUser(address);

      const subscriptions = await subscriptionRepo.find({
        where: { createdBy: user.id },
        order: { createdAt: "DESC" },
      });

      // Do not return secret keys in the list
      const sanitized = subscriptions.map((sub) => ({
        id: sub.id,
        targetUrl: sub.targetUrl,
        events: sub.events,
        isActive: sub.isActive,
        failureCount: sub.failureCount,
        maxRetries: sub.maxRetries,
        communityId: sub.communityId,
        createdAt: sub.createdAt,
        updatedAt: sub.updatedAt,
      }));

      res.json({ data: sanitized });
    } catch (error: any) {
      if (error.message === "Authentication required") {
        res.status(401).json({ error: "Authentication required" });
        return;
      }
      next(error);
    }
  });

  /**
   * POST /webhooks
   * Create a new webhook subscription
   */
  router.post("/", validateBody(webhookSubscriptionCreateSchema), async (req: AuthenticatedRequest, res, next) => {
    try {
      const address = getCallerAddress(req);
      const user = await getOrCreateUser(address);
      const body = (req as any).validatedBody;

      const subscription = subscriptionRepo.create({
        targetUrl: body.targetUrl,
        secretKey: body.secretKey,
        events: body.events,
        createdBy: user.id,
        communityId: body.communityId ?? null,
        ownerAddress: address,
        isActive: true,
        failureCount: 0,
        maxRetries: 5,
      });

      const saved = await subscriptionRepo.save(subscription);

      res.status(201).json({
        data: {
          id: saved.id,
          targetUrl: saved.targetUrl,
          events: saved.events,
          isActive: saved.isActive,
          createdAt: saved.createdAt,
        },
      });
    } catch (error: any) {
      if (error.message === "Authentication required") {
        res.status(401).json({ error: "Authentication required" });
        return;
      }
      next(error);
    }
  });

  /**
   * GET /webhooks/:id
   * Get a single subscription by ID
   */
  router.get("/:id", validateParams(idParamSchema), async (req: AuthenticatedRequest, res, next) => {
    try {
      const { id } = (req as any).validatedParams;
      const address = getCallerAddress(req);
      const user = await getOrCreateUser(address);

      const subscription = await subscriptionRepo.findOne({
        where: { id, createdBy: user.id },
      });

      if (!subscription) {
        res.status(404).json({ error: "Subscription not found" });
        return;
      }

      res.json({
        data: {
          id: subscription.id,
          targetUrl: subscription.targetUrl,
          events: subscription.events,
          isActive: subscription.isActive,
          failureCount: subscription.failureCount,
          maxRetries: subscription.maxRetries,
          communityId: subscription.communityId,
          createdAt: subscription.createdAt,
          updatedAt: subscription.updatedAt,
        },
      });
    } catch (error: any) {
      if (error.message === "Authentication required") {
        res.status(401).json({ error: "Authentication required" });
        return;
      }
      next(error);
    }
  });

  /**
   * PATCH /webhooks/:id
   * Update a subscription
   */
  router.patch("/:id", validateRequest({ params: idParamSchema, body: webhookSubscriptionUpdateSchema }), async (req: AuthenticatedRequest, res, next) => {
    try {
      const { id } = (req as any).validatedParams;
      const address = getCallerAddress(req);
      const user = await getOrCreateUser(address);
      const body = (req as any).validatedBody;

      const subscription = await subscriptionRepo.findOne({
        where: { id, createdBy: user.id },
      });

      if (!subscription) {
        res.status(404).json({ error: "Subscription not found" });
        return;
      }

      if (body.targetUrl !== undefined) subscription.targetUrl = body.targetUrl;
      if (body.secretKey !== undefined) subscription.secretKey = body.secretKey;
      if (body.events !== undefined) subscription.events = body.events;
      if (body.isActive !== undefined) subscription.isActive = body.isActive;

      const saved = await subscriptionRepo.save(subscription);

      res.json({
        data: {
          id: saved.id,
          targetUrl: saved.targetUrl,
          events: saved.events,
          isActive: saved.isActive,
          updatedAt: saved.updatedAt,
        },
      });
    } catch (error: any) {
      if (error.message === "Authentication required") {
        res.status(401).json({ error: "Authentication required" });
        return;
      }
      next(error);
    }
  });

  /**
   * DELETE /webhooks/:id
   * Delete a subscription
   */
  router.delete("/:id", validateParams(idParamSchema), async (req: AuthenticatedRequest, res, next) => {
    try {
      const { id } = (req as any).validatedParams;
      const address = getCallerAddress(req);
      const user = await getOrCreateUser(address);

      const result = await subscriptionRepo.delete({ id, createdBy: user.id });

      if (result.affected === 0) {
        res.status(404).json({ error: "Subscription not found" });
        return;
      }

      res.json({ ok: true });
    } catch (error: any) {
      if (error.message === "Authentication required") {
        res.status(401).json({ error: "Authentication required" });
        return;
      }
      next(error);
    }
  });

  // ---------------------------------------------------------------------------
  // Delivery logs
  // ---------------------------------------------------------------------------

  /**
   * GET /webhooks/:id/logs
   * Get delivery logs for a subscription
   */
  router.get("/:id/logs", validateParams(idParamSchema), async (req: AuthenticatedRequest, res, next) => {
    try {
      const { id } = (req as any).validatedParams;
      const address = getCallerAddress(req);
      const user = await getOrCreateUser(address);

      const subscription = await subscriptionRepo.findOne({
        where: { id, createdBy: user.id },
      });

      if (!subscription) {
        res.status(404).json({ error: "Subscription not found" });
        return;
      }

      const page = Math.max(1, Number(req.query.page ?? "1"));
      const limit = Math.min(100, Math.max(1, Number(req.query.limit ?? "20")));
      const offset = (page - 1) * limit;

      const { logs, total } = await webhookDispatcher.getDeliveryLogs(id, limit, offset);

      res.json({
        data: logs,
        pagination: { page, limit, total, pages: Math.ceil(total / limit) },
      });
    } catch (error: any) {
      if (error.message === "Authentication required") {
        res.status(401).json({ error: "Authentication required" });
        return;
      }
      next(error);
    }
  });

  // ---------------------------------------------------------------------------
  // Test endpoint
  // ---------------------------------------------------------------------------

  /**
   * POST /webhooks/test
   * Send a test event to a target URL (no DB persistence)
   */
  router.post("/test", validateBody(webhookTestSchema), async (req: AuthenticatedRequest, res, next) => {
    try {
      const body = (req as any).validatedBody;

      const payload: WebhookPayload = {
        event: body.event,
        timestamp: new Date().toISOString(),
        data: { message: "This is a test event from StellarGrant" },
      };

      const signature = WebhookDispatcher.signPayload(payload, body.secretKey);

      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 30000);

      const response = await fetch(body.targetUrl, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Webhook-Signature": `sha256=${signature}`,
          "X-Webhook-Event": body.event,
          "X-Webhook-Timestamp": payload.timestamp,
          "User-Agent": "StellarGrant-Webhook/1.0",
        },
        body: JSON.stringify(payload),
        signal: controller.signal,
      });

      clearTimeout(timeout);

      const responseBody = await response.text();

      res.json({
        ok: response.ok,
        statusCode: response.status,
        headers: Object.fromEntries(response.headers.entries()),
        body: responseBody.slice(0, 2000),
        sentPayload: payload,
        sentSignature: signature,
      });
    } catch (error: any) {
      res.status(502).json({
        ok: false,
        error: error.message || "Failed to reach target URL",
        sentPayload: {
          event: (req as any).validatedBody?.event,
          timestamp: new Date().toISOString(),
          data: { message: "This is a test event from StellarGrant" },
        },
      });
    }
  });

  // ---------------------------------------------------------------------------
  // Admin endpoint to list all subscriptions (admin only)
  // ---------------------------------------------------------------------------

  /**
   * GET /webhooks/admin/all
   * List all subscriptions across all users (admin only)
   */
  router.get(
    "/admin/all",
    requirePermission("admin:all" as Permission),
    async (_req, res, next) => {
      try {
        const [subscriptions, total] = await subscriptionRepo.findAndCount({
          order: { createdAt: "DESC" },
          take: 100,
        });

        const sanitized = subscriptions.map((sub) => ({
          id: sub.id,
          targetUrl: sub.targetUrl,
          events: sub.events,
          isActive: sub.isActive,
          failureCount: sub.failureCount,
          ownerAddress: sub.ownerAddress,
          communityId: sub.communityId,
          createdAt: sub.createdAt,
        }));

        res.json({ data: sanitized, total });
      } catch (error) {
        next(error);
      }
    }
  );

  return router;
};
