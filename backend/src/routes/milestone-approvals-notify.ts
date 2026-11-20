import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { MilestoneApproval } from "../entities/MilestoneApproval";
import { Grant } from "../entities/Grant";
import { GrantReviewer } from "../entities/GrantReviewer";
import { User } from "../entities/User";
import { getEmailTemplate, sendEmail } from "../services/email-service";
import { WebhookDispatcher } from "../services/webhook-dispatcher";
import { WebhookEventType } from "../entities/WebhookSubscription";
import { SignatureService } from "../services/signature-service";
import { NonceService, NonceReplayError } from "../services/nonce-service";
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

export const buildMilestoneApprovalNotifyRouter = (
  approvalRepo: Repository<MilestoneApproval>,
  grantRepo: Repository<Grant>,
  userRepo: Repository<User>,
  reviewerRepo: Repository<GrantReviewer>,
  signatureService: SignatureService,
  nonceService: NonceService,
  webhookDispatcher?: WebhookDispatcher,
) => {
  const router = Router();

  // Reviewer approves a milestone and triggers notification to grant owner
  // Requires rate limiting (#1251) to prevent notification spam
  router.post("/notify", walletLimiters.milestoneApproval, async (req, res, next) => {
    try {
      const parsed = approvalSchema.safeParse(req.body);
      if (!parsed.success) {
        return res.status(400).json({ error: "Invalid payload", details: parsed.error.issues });
      }
      const { grantId, milestoneIdx, reviewerStellarAddress, approved } = parsed.data;
      if (Math.abs(Date.now() - parsed.data.timestamp) > 5 * 60 * 1000) {
        return res.status(400).json({ error: "Expired intent timestamp" });
      }
      if (!signatureService.verifyMilestoneApproval(parsed.data)) {
        return res.status(401).json({ error: "Invalid Stellar signature" });
      }

      const grant = await grantRepo.findOne({ where: { id: grantId } });
      if (!grant) {
        return res.status(404).json({ error: "Grant not found" });
      }
      const reviewer = await reviewerRepo.findOne({ where: { grantId, reviewerStellarAddress } });
      if (!reviewer) {
        return res.status(403).json({ error: "Reviewer is not registered for this grant" });
      }
      await nonceService.consume(parsed.data.nonce);

      // Check for duplicate approval (dedup) (#1251)
      const existing = await approvalRepo.findOne({
        where: { grantId, milestoneIdx, reviewerStellarAddress },
      });
      if (existing) {
        return res.status(409).json({ error: "Duplicate approval: this reviewer has already voted on this milestone" });
      }

      const approval = approvalRepo.create({ grantId, milestoneIdx, reviewerStellarAddress, approved });
      await approvalRepo.save(approval);

      // Notify grant owner if approved
      if (approved) {
        const owner = await userRepo.findOne({ where: { stellarAddress: grant.recipient } });
        if (owner && owner.email && owner.notifyMilestoneApproved) {
          const emailData = {
            grantTitle: grant.title,
            milestoneTitle: `#${milestoneIdx}`,
          };
          const { subject, html } = getEmailTemplate('milestone_approved', emailData);
          await sendEmail({ to: owner.email, subject, html });
        }

        // Dispatch webhook event for milestone approved
        webhookDispatcher?.dispatch(WebhookEventType.MILESTONE_APPROVED, {
          grantId,
          milestoneIdx,
          reviewerStellarAddress,
          approved,
        });
      } else {
        // Dispatch webhook event for milestone rejected
        webhookDispatcher?.dispatch(WebhookEventType.MILESTONE_REJECTED, {
          grantId,
          milestoneIdx,
          reviewerStellarAddress,
          approved,
        });
      }

      return res.status(201).json({ data: approval });
    } catch (error) {
      if (error instanceof NonceReplayError) {
        return res.status(409).json({ error: "Signature nonce has already been used" });
      }
      return next(error);
    }
  });

  return router;
};
