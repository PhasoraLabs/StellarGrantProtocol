import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { MilestoneProof } from "../entities/MilestoneProof";
import { SignatureService } from "../services/signature-service";
import { getEmailTemplate, sendEmail } from "../services/email-service";
import { Grant } from "../entities/Grant";
import { User } from "../entities/User";
import { walletLimiters } from "../middlewares/rate-limiter";

const milestoneProofSchema = z.object({
  grantId: z.number().int().positive(),
  milestoneIdx: z.number().int().nonnegative(),
  proofCid: z.string().min(3).max(255),
  submittedBy: z.string().min(10).max(120),
  signature: z.string().min(32),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
});

export const buildMilestoneProofRouter = (
  proofRepo: Repository<MilestoneProof>,
  signatureService: SignatureService,
  grantRepo?: Repository<Grant>,
  userRepo?: Repository<User>,
) => {
  const router = Router();

  router.post("/", walletLimiters.milestoneProof, async (req, res, next) => {
    try {
      const parsed = milestoneProofSchema.safeParse(req.body);
      if (!parsed.success) {
        res.status(400).json({ error: "Invalid payload", details: parsed.error.issues });
        return;
      }

      const payload = parsed.data;
      const maxSkewMs = 5 * 60 * 1000;
      if (Math.abs(Date.now() - payload.timestamp) > maxSkewMs) {
        res.status(400).json({ error: "Expired intent timestamp" });
        return;
      }

      const signatureIsValid = signatureService.verify(payload);
      if (!signatureIsValid) {
        res.status(401).json({ error: "Invalid Stellar signature" });
        return;
      }

      const proof = await proofRepo.save({
        grantId: payload.grantId,
        milestoneIdx: payload.milestoneIdx,
        proofCid: payload.proofCid,
        submittedBy: payload.submittedBy,
        signature: payload.signature,
        nonce: payload.nonce,
      });

      // Send emails if repos are available
      if (grantRepo && userRepo) {
        const grant = await grantRepo.findOne({
          where: { id: payload.grantId },
          relations: ["reviewers"],
        });
        if (grant) {
          // Notify owner
          const owner = await userRepo.findOne({ where: { stellarAddress: grant.recipient } });
          if (owner && owner.email && owner.notifyMilestoneSubmitted !== false) {
            const emailData = {
              grantTitle: grant.title,
              milestoneTitle: `#${payload.milestoneIdx}`,
            };
            const { subject, html } = getEmailTemplate("milestone_submitted", emailData);
            await sendEmail({ to: owner.email, subject, html });
          }

          // Notify reviewers
          if (grant.reviewers && grant.reviewers.length > 0) {
            for (const reviewer of grant.reviewers) {
              const user = await userRepo.findOne({ where: { stellarAddress: reviewer.reviewerStellarAddress } });
              if (user && user.email && user.notifyMilestoneSubmitted !== false) {
                const emailData = {
                  grantTitle: grant.title,
                  milestoneTitle: `#${payload.milestoneIdx}`,
                };
                const { subject, html } = getEmailTemplate("milestone_submitted", emailData);
                await sendEmail({ to: user.email, subject, html });
              }
            }
          }
        }
      }

      res.status(201).json({ data: proof });
    } catch (error: any) {
      if (error?.code === "23505" || error?.code === "SQLITE_CONSTRAINT") {
        res.status(409).json({ error: "Proof already submitted for this milestone" });
        return;
      }
      next(error);
    }
  });

  return router;
};
