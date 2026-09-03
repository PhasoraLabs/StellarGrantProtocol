import { Router } from "express";
import { Repository } from "typeorm";
import { MilestoneApproval } from "../entities/MilestoneApproval";
import { Grant } from "../entities/Grant";
import { User } from "../entities/User";
import { getEmailTemplate, sendEmail } from "../services/email-service";
import { WebhookDispatcher } from "../services/webhook-dispatcher";
import { WebhookEventType } from "../entities/WebhookSubscription";

export const buildMilestoneApprovalNotifyRouter = (
  approvalRepo: Repository<MilestoneApproval>,
  grantRepo: Repository<Grant>,
  userRepo: Repository<User>,
  webhookDispatcher?: WebhookDispatcher,
) => {
  const router = Router();

  // Reviewer approves a milestone and triggers notification to grant owner
  router.post("/notify", async (req, res) => {
    const { grantId, milestoneIdx, reviewerStellarAddress, approved } = req.body;
    if (!grantId || milestoneIdx === undefined || !reviewerStellarAddress || approved === undefined) {
      return res.status(400).json({ error: "grantId, milestoneIdx, reviewerStellarAddress, and approved are required" });
    }
    const approval = approvalRepo.create({ grantId, milestoneIdx, reviewerStellarAddress, approved });
    await approvalRepo.save(approval);

    // Notify grant owner if approved
    if (approved) {
      const grant = await grantRepo.findOne({ where: { id: grantId } });
      if (grant) {
        const owner = await userRepo.findOne({ where: { stellarAddress: grant.recipient } });
        if (owner && owner.email && owner.notifyMilestoneApproved) {
          const emailData = {
            grantTitle: grant.title,
            milestoneTitle: `#${milestoneIdx}`,
          };
          const { subject, html } = getEmailTemplate('milestone_approved', emailData);
          await sendEmail({ to: owner.email, subject, html });
        }
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

    res.json({ data: approval });
  });

  return router;
};
