import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { MilestoneApproval } from "../entities/MilestoneApproval";
import { Grant } from "../entities/Grant";
import { GrantReviewer } from "../entities/GrantReviewer";
import { SignatureService } from "../services/signature-service";
import { NonceService, NonceReplayError } from "../services/nonce-service";
import { notificationService } from "../services/notification-service";
import { walletLimiters } from "../middlewares/rate-limiter";

const approvalSchema = z.object({
  grantId: z.number().int().positive(),
  milestoneIdx: z.number().int().nonnegative(),
  reviewerStellarAddress: z.string().regex(/^G[A-Z2-7]{55}$/),
  approved: z.boolean(),
  address: z.string().regex(/^G[A-Z2-7]{55}$/),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
  signature: z.string().min(32),
});

export const buildMilestoneApprovalRouter = (
  approvalRepo: Repository<MilestoneApproval>,
  grantRepo: Repository<Grant>,
  reviewerRepo: Repository<GrantReviewer>,
  signatureService: SignatureService,
  nonceService: NonceService,
) => {
  const router = Router();

  // Reviewer approves or rejects a milestone
  router.post("/", walletLimiters.milestoneApproval, async (req, res, next) => {
    try {
      const parsed = approvalSchema.safeParse(req.body);
      if (!parsed.success) {
        return res.status(400).json({ error: "Invalid payload", details: parsed.error.issues });
      }
      const payload = parsed.data;

      if (Math.abs(Date.now() - payload.timestamp) > 5 * 60 * 1000) {
        return res.status(400).json({ error: "Expired intent timestamp" });
      }
      if (!signatureService.verifyMilestoneApproval(payload)) {
        return res.status(401).json({ error: "Invalid Stellar signature" });
      }

      const grant = await grantRepo.findOne({ where: { id: payload.grantId } });
      if (!grant) {
        return res.status(404).json({ error: "Grant not found" });
      }
      const reviewer = await reviewerRepo.findOne({
        where: {
          grantId: payload.grantId,
          reviewerStellarAddress: payload.reviewerStellarAddress,
        },
      });
      if (!reviewer) {
        return res.status(403).json({ error: "Reviewer is not registered for this grant" });
      }

      await nonceService.consume(payload.nonce);

      const existing = await approvalRepo.findOne({
        where: {
          grantId: payload.grantId,
          milestoneIdx: payload.milestoneIdx,
          reviewerStellarAddress: payload.reviewerStellarAddress,
        },
      });
      const approval = approvalRepo.create({
        ...(existing ?? {}),
        grantId: payload.grantId,
        milestoneIdx: payload.milestoneIdx,
        reviewerStellarAddress: payload.reviewerStellarAddress,
        approved: payload.approved,
      });
      const saved = await approvalRepo.save(approval);
      try {
        notificationService.broadcast("milestoneApproved", {
          grantId: payload.grantId,
          milestoneIdx: payload.milestoneIdx,
          approved: payload.approved,
          reviewer: payload.reviewerStellarAddress,
        });
      } catch (error) {
        console.warn("Failed to broadcast milestone approval", error);
      }
      return res.status(existing ? 200 : 201).json({ data: saved });
    } catch (error: any) {
      if (error instanceof NonceReplayError) {
        return res.status(409).json({ error: "Signature nonce has already been used" });
      }
      if (error?.code === "23505" || error?.code === "SQLITE_CONSTRAINT") {
        return res.status(409).json({ error: "Duplicate approval" });
      }
      return next(error);
    }
  });

  // List approvals for a milestone
  router.get("/grant/:grantId/milestone/:milestoneIdx", async (req, res) => {
    const { grantId, milestoneIdx } = req.params;
    const approvals = await approvalRepo.find({ where: { grantId: Number(grantId), milestoneIdx: Number(milestoneIdx) } });
    res.json({ data: approvals });
  });

  return router;
};
