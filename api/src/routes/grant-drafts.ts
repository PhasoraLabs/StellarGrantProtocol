import { Router } from "express";
import { Repository, DataSource } from "typeorm";
import { z } from "zod";
import { Keypair, StrKey } from "@stellar/stellar-sdk";
import { Grant } from "../entities/Grant";
import { WebhookDispatcher } from "../services/webhook-dispatcher";
import { validateBody, validateParams } from "../middlewares/validation-middleware";
import { stellarAddressSchema, idParamSchema } from "../schemas";

const MAX_SKEW_MS = 5 * 60 * 1000;

const draftSchema = z.object({
  title: z.string().min(10).max(120).optional(),
  description: z.string().optional(),
  recipientAddress: z.string().regex(/^G[A-Z2-7]{55}$/).optional(),
  totalBudget: z.number().positive().optional(),
  budgetToken: z.enum(["native", "USDC"]).optional(),
  milestones: z.array(z.object({ title: z.string(), reward: z.number().optional() })).optional(),
  reviewers: z.array(z.string().regex(/^G[A-Z2-7]{55}$/)).optional(),
  quorum: z.number().int().min(1).optional(),
}).partial();

const draftCreateSchema = z.object({
  address: stellarAddressSchema,
  signature: z.string().min(32),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
  draft: draftSchema,
});

const draftUpdateSchema = z.object({
  address: stellarAddressSchema,
  signature: z.string().min(32),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
  draft: draftSchema,
});

const draftPublishSchema = z.object({
  address: stellarAddressSchema,
  signature: z.string().min(32),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
});

function buildDraftIntentMessage(payload: {
  address: string;
  nonce: string;
  timestamp: number;
  action: string;
  draftId?: number;
}): string {
  const parts = [
    "stellargrant:draft:v1",
    payload.address,
    payload.nonce,
    payload.timestamp,
    payload.action,
  ];
  if (payload.draftId !== undefined) {
    parts.push(payload.draftId.toString());
  }
  return parts.join("|");
}

function verifySignature(params: {
  address: string;
  signature: string;
  message: string;
}): boolean {
  if (!StrKey.isValidEd25519PublicKey(params.address)) return false;
  const keypair = Keypair.fromPublicKey(params.address);
  return keypair.verify(
    Buffer.from(params.message, "utf8"),
    Buffer.from(params.signature, "base64"),
  );
}

function calculateCompleteness(draftData: Record<string, unknown>): number {
  const requiredFields = ["title", "description", "recipientAddress", "totalBudget", "budgetToken", "milestones", "reviewers", "quorum"];
  const filledFields = requiredFields.filter((field) => {
    const value = draftData[field];
    if (value === undefined || value === null) return false;
    if (Array.isArray(value) && value.length === 0) return false;
    if (typeof value === "string" && value.trim() === "") return false;
    return true;
  });
  return filledFields.length / requiredFields.length;
}

export const buildGrantDraftsRouter = (
  dataSource: DataSource,
  webhookDispatcher: WebhookDispatcher,
) => {
  const router = Router();
  const grantRepo: Repository<Grant> = dataSource.getRepository(Grant);

  // POST /grants/draft - create a new draft grant
  router.post("/draft", validateBody(draftCreateSchema), async (req, res, next) => {
    try {
      const { address, signature, nonce, timestamp, draft } = (req as any).validatedBody;

      if (Math.abs(Date.now() - timestamp) > MAX_SKEW_MS) {
        res.status(400).json({ error: "Expired intent timestamp" });
        return;
      }

      const message = buildDraftIntentMessage({ address, nonce, timestamp, action: "create" });
      const ok = verifySignature({ address, signature, message });
      if (!ok) {
        res.status(401).json({ error: "Invalid signature" });
        return;
      }

      const grant = grantRepo.create({
        title: draft.title || "",
        description: draft.description || null,
        recipient: draft.recipientAddress || address,
        totalAmount: draft.totalBudget ? draft.totalBudget.toString() : "0",
        status: "draft",
        isDraft: true,
        draftData: draft,
        owner: address,
      });

      const saved = await grantRepo.save(grant);
      const completeness = calculateCompleteness(saved.draftData || {});

      res.status(201).json({
        data: {
          draftId: saved.id,
          savedAt: saved.updatedAt,
          completeness,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  // GET /grants/draft/:id - get a draft by ID (owner only)
  router.get("/draft/:id", validateParams(idParamSchema), async (req, res, next) => {
    try {
      const { id } = (req as any).validatedParams;

      const grant = await grantRepo.findOne({ where: { id } });
      if (!grant) {
        res.status(404).json({ error: "Draft not found" });
        return;
      }

      if (!grant.isDraft) {
        res.status(400).json({ error: "Not a draft" });
        return;
      }

      const authHeader = req.header("authorization");
      if (!authHeader || !authHeader.startsWith("Bearer ")) {
        res.status(401).json({ error: "Authorization required" });
        return;
      }

      const token = authHeader.replace("Bearer ", "");
      // For simplicity, we'll use the address from the token or query param
      // In production, you'd verify the JWT token
      const address = req.query.address as string || token;

      if (address !== grant.owner && address !== grant.recipient) {
        res.status(403).json({ error: "Access denied" });
        return;
      }

      const completeness = calculateCompleteness(grant.draftData || {});

      res.json({
        data: {
          id: grant.id,
          draftData: grant.draftData,
          completeness,
          savedAt: grant.updatedAt,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  // PATCH /grants/draft/:id - update draft fields
  router.patch("/draft/:id", validateParams(idParamSchema), validateBody(draftUpdateSchema), async (req, res, next) => {
    try {
      const { id } = (req as any).validatedParams;
      const { address, signature, nonce, timestamp, draft } = (req as any).validatedBody;

      if (Math.abs(Date.now() - timestamp) > MAX_SKEW_MS) {
        res.status(400).json({ error: "Expired intent timestamp" });
        return;
      }

      const message = buildDraftIntentMessage({ address, nonce, timestamp, action: "update", draftId: id });
      const ok = verifySignature({ address, signature, message });
      if (!ok) {
        res.status(401).json({ error: "Invalid signature" });
        return;
      }

      const grant = await grantRepo.findOne({ where: { id } });
      if (!grant) {
        res.status(404).json({ error: "Draft not found" });
        return;
      }

      if (!grant.isDraft) {
        res.status(400).json({ error: "Not a draft" });
        return;
      }

      if (address !== grant.owner && address !== grant.recipient) {
        res.status(403).json({ error: "Access denied" });
        return;
      }

      // Deep merge draftData
      const currentDraftData = (grant.draftData || {}) as Record<string, unknown>;
      const mergedDraftData = { ...currentDraftData, ...draft };

      grant.draftData = mergedDraftData;
      const saved = await grantRepo.save(grant);
      const completeness = calculateCompleteness(saved.draftData || {});

      res.json({
        data: {
          draftId: saved.id,
          savedAt: saved.updatedAt,
          completeness,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  // POST /grants/draft/:id/publish - publish the draft
  router.post("/draft/:id/publish", validateParams(idParamSchema), validateBody(draftPublishSchema), async (req, res, next) => {
    try {
      const { id } = (req as any).validatedParams;
      const { address, signature, nonce, timestamp } = (req as any).validatedBody;

      if (Math.abs(Date.now() - timestamp) > MAX_SKEW_MS) {
        res.status(400).json({ error: "Expired intent timestamp" });
        return;
      }

      const message = buildDraftIntentMessage({ address, nonce, timestamp, action: "publish", draftId: id });
      const ok = verifySignature({ address, signature, message });
      if (!ok) {
        res.status(401).json({ error: "Invalid signature" });
        return;
      }

      const grant = await grantRepo.findOne({ where: { id } });
      if (!grant) {
        res.status(404).json({ error: "Draft not found" });
        return;
      }

      if (!grant.isDraft) {
        res.status(400).json({ error: "Not a draft" });
        return;
      }

      if (address !== grant.owner && address !== grant.recipient) {
        res.status(403).json({ error: "Access denied" });
        return;
      }

      const completeness = calculateCompleteness(grant.draftData || {});
      if (completeness < 1.0) {
        res.status(400).json({ error: "Draft is incomplete. All required fields must be filled." });
        return;
      }

      // Return unsigned XDR for the frontend to sign
      // In a real implementation, this would call the Soroban client to build the transaction
      // For now, we'll return a placeholder
      const unsignedXdr = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"; // Placeholder

      res.json({
        data: {
          unsignedXdr,
          draftId: grant.id,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  // DELETE /grants/draft/:id - delete a draft
  router.delete("/draft/:id", validateParams(idParamSchema), async (req, res, next) => {
    try {
      const { id } = (req as any).validatedParams;

      const authHeader = req.header("authorization");
      if (!authHeader || !authHeader.startsWith("Bearer ")) {
        res.status(401).json({ error: "Authorization required" });
        return;
      }

      const token = authHeader.replace("Bearer ", "");
      const address = req.query.address as string || token;

      const grant = await grantRepo.findOne({ where: { id } });
      if (!grant) {
        res.status(404).json({ error: "Draft not found" });
        return;
      }

      if (!grant.isDraft) {
        res.status(400).json({ error: "Not a draft" });
        return;
      }

      if (address !== grant.owner && address !== grant.recipient) {
        res.status(403).json({ error: "Access denied" });
        return;
      }

      await grantRepo.remove(grant);

      res.status(204).send();
    } catch (error) {
      next(error);
    }
  });

  // GET /users/:address/drafts - list all drafts for this address
  router.get("/users/:address/drafts", async (req, res, next) => {
    try {
      const { address } = req.params;

      if (!StrKey.isValidEd25519PublicKey(address)) {
        res.status(400).json({ error: "Invalid Stellar address" });
        return;
      }

      const drafts = await grantRepo.find({
        where: { isDraft: true, owner: address },
        order: { updatedAt: "DESC" },
      });

      const draftsWithCompleteness = drafts.map((draft) => ({
        id: draft.id,
        title: draft.draftData?.title || draft.title,
        completeness: calculateCompleteness(draft.draftData || {}),
        savedAt: draft.updatedAt,
      }));

      res.json({ data: draftsWithCompleteness });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
