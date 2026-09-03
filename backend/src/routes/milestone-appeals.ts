import { Router } from "express";
import { Repository, DataSource } from "typeorm";
import { z } from "zod";
import { Keypair, StrKey } from "@stellar/stellar-sdk";
import { MilestoneAppeal } from "../entities/MilestoneAppeal";
import { Grant } from "../entities/Grant";
import { MilestoneProof } from "../entities/MilestoneProof";
import { GrantReviewer } from "../entities/GrantReviewer";
import { MilestoneApproval } from "../entities/MilestoneApproval";
import { Activity } from "../entities/Activity";
import { WebhookDispatcher } from "../services/webhook-dispatcher";
import { NotificationService } from "../services/notification-service";
import { validateBody, validateParams } from "../middlewares/validation-middleware";
import { stellarAddressSchema, paginationSchema } from "../schemas";

const MAX_SKEW_MS = 5 * 60 * 1000;

const appealOpenSchema = z.object({
  address: stellarAddressSchema,
  reason: z.string().min(1).max(5000),
  signature: z.string().min(32),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
});

const appealVoteSchema = z.object({
  address: stellarAddressSchema,
  uphold: z.boolean(),
  signature: z.string().min(32),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
});

const grantMilestoneParamsSchema = z.object({
  id: z.coerce.number().int().positive(),
  idx: z.coerce.number().int().nonnegative(),
});

function buildAppealIntentMessage(payload: {
  grantId: number;
  milestoneIdx: number;
  reason: string;
  address: string;
  nonce: string;
  timestamp: number;
}): string {
  return [
    "stellargrant:appeal:v1",
    payload.grantId,
    payload.milestoneIdx,
    payload.reason,
    payload.address,
    payload.nonce,
    payload.timestamp,
  ].join("|");
}

function buildAppealVoteIntentMessage(payload: {
  grantId: number;
  milestoneIdx: number;
  reviewer: string;
  uphold: boolean;
  nonce: string;
  timestamp: number;
}): string {
  return [
    "stellargrant:appeal_vote:v1",
    payload.grantId,
    payload.milestoneIdx,
    payload.reviewer,
    payload.uphold.toString(),
    payload.nonce,
    payload.timestamp,
  ].join("|");
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

export const buildMilestoneAppealsRouter = (
  dataSource: DataSource,
  webhookDispatcher: WebhookDispatcher,
  notificationService: NotificationService,
) => {
  const router = Router({ mergeParams: true });
  const appealRepo: Repository<MilestoneAppeal> = dataSource.getRepository(MilestoneAppeal);
  const grantRepo: Repository<Grant> = dataSource.getRepository(Grant);
  const proofRepo: Repository<MilestoneProof> = dataSource.getRepository(MilestoneProof);
  const reviewerRepo: Repository<GrantReviewer> = dataSource.getRepository(GrantReviewer);
  const approvalRepo: Repository<MilestoneApproval> = dataSource.getRepository(MilestoneApproval);
  const activityRepo: Repository<Activity> = dataSource.getRepository(Activity);

  // POST /grants/:id/milestones/:idx/appeal - open an appeal
  router.post(
    "/grants/:id/milestones/:idx/appeal",
    validateParams(grantMilestoneParamsSchema),
    validateBody(appealOpenSchema),
    async (req, res, next) => {
      try {
        const { id: grantId, idx: milestoneIdx } = (req as any).validatedParams;
        const { address, reason, signature, nonce, timestamp } = (req as any).validatedBody;

        if (Math.abs(Date.now() - timestamp) > MAX_SKEW_MS) {
          res.status(400).json({ error: "Expired intent timestamp" });
          return;
        }

        const message = buildAppealIntentMessage({ grantId, milestoneIdx, reason, address, nonce, timestamp });
        const ok = verifySignature({ address, signature, message });
        if (!ok) {
          res.status(401).json({ error: "Invalid signature" });
          return;
        }

        const grant = await grantRepo.findOne({ where: { id: grantId } });
        if (!grant) {
          res.status(404).json({ error: "Grant not found" });
          return;
        }

        if (address !== grant.recipient) {
          res.status(403).json({ error: "Only the grant recipient can open an appeal" });
          return;
        }

        // Check if milestone is rejected (has MilestoneApproval with approved: false)
        const approvals = await approvalRepo.find({
          where: { grantId, milestoneIdx },
        });
        const hasRejection = approvals.some((a) => a.approved === false);
        if (!hasRejection) {
          res.status(400).json({ error: "Milestone must be rejected to open an appeal" });
          return;
        }

        // Check for existing appeal
        const existing = await appealRepo.findOne({
          where: { grantId, milestoneIdx },
        });
        if (existing) {
          res.status(409).json({ error: "Appeal already exists for this milestone" });
          return;
        }

        const appeal = appealRepo.create({
          grantId,
          milestoneIdx,
          reason,
          appellantAddress: address,
          status: "pending",
          reviewerVotes: [],
        });
        const saved = await appealRepo.save(appeal);

        // Notify all reviewers
        const reviewers = await reviewerRepo.find({ where: { grantId } });
        reviewers.forEach((r) => {
          notificationService.notifyUser(r.reviewerStellarAddress, "milestone_appealed", {
            grantId,
            milestoneIdx,
            appealId: saved.id,
            reason,
          });
        });

        // Fire webhook
        await webhookDispatcher
          .dispatch("milestone.appealed" as never, {
            appealId: saved.id,
            grantId,
            milestoneIdx,
            appellantAddress: address,
            reason,
          })
          .catch(() => {});

        res.status(201).json({ data: saved });
      } catch (error) {
        next(error);
      }
    },
  );

  // GET /grants/:id/milestones/:idx/appeal - get appeal status
  router.get(
    "/grants/:id/milestones/:idx/appeal",
    validateParams(grantMilestoneParamsSchema),
    async (req, res, next) => {
      try {
        const { id: grantId, idx: milestoneIdx } = (req as any).validatedParams;

        const appeal = await appealRepo.findOne({
          where: { grantId, milestoneIdx },
        });

        if (!appeal) {
          res.status(404).json({ error: "Appeal not found" });
          return;
        }

        const reviewers = await reviewerRepo.find({ where: { grantId } });
        const totalReviewers = reviewers.length;
        const votesFor = appeal.reviewerVotes.filter((v) => v.uphold).length;
        const votesAgainst = appeal.reviewerVotes.filter((v) => !v.uphold).length;
        const votesRemaining = totalReviewers - appeal.reviewerVotes.length;

        res.json({
          data: {
            status: appeal.status,
            reason: appeal.reason,
            openedAt: appeal.openedAt,
            resolvedAt: appeal.resolvedAt,
            votesFor,
            votesAgainst,
            votesRemaining,
          },
        });
      } catch (error) {
        next(error);
      }
    },
  );

  // POST /grants/:id/milestones/:idx/appeal/vote - reviewer votes on appeal
  router.post(
    "/grants/:id/milestones/:idx/appeal/vote",
    validateParams(grantMilestoneParamsSchema),
    validateBody(appealVoteSchema),
    async (req, res, next) => {
      try {
        const { id: grantId, idx: milestoneIdx } = (req as any).validatedParams;
        const { address: reviewer, uphold, signature, nonce, timestamp } = (req as any).validatedBody;

        if (Math.abs(Date.now() - timestamp) > MAX_SKEW_MS) {
          res.status(400).json({ error: "Expired intent timestamp" });
          return;
        }

        const message = buildAppealVoteIntentMessage({ grantId, milestoneIdx, reviewer, uphold, nonce, timestamp });
        const ok = verifySignature({ address: reviewer, signature, message });
        if (!ok) {
          res.status(401).json({ error: "Invalid signature" });
          return;
        }

        const appeal = await appealRepo.findOne({
          where: { grantId, milestoneIdx },
        });
        if (!appeal) {
          res.status(404).json({ error: "Appeal not found" });
          return;
        }

        if (appeal.status !== "pending") {
          res.status(400).json({ error: "Appeal is already resolved" });
          return;
        }

        // Validate reviewer is in GrantReviewer
        const reviewerRecord = await reviewerRepo.findOne({
          where: { grantId, reviewerStellarAddress: reviewer },
        });
        if (!reviewerRecord) {
          res.status(403).json({ error: "Not a reviewer for this grant" });
          return;
        }

        // Prevent duplicate votes
        const hasVoted = appeal.reviewerVotes.some((v) => v.reviewer === reviewer);
        if (hasVoted) {
          res.status(409).json({ error: "Already voted on this appeal" });
          return;
        }

        // Add vote
        appeal.reviewerVotes.push({
          reviewer,
          uphold,
          votedAt: new Date().toISOString(),
        });
        await appealRepo.save(appeal);

        // Check if all reviewers have voted
        const allReviewers = await reviewerRepo.find({ where: { grantId } });
        if (appeal.reviewerVotes.length === allReviewers.length) {
          // Auto-resolve appeal
          const upholdCount = appeal.reviewerVotes.filter((v) => v.uphold).length;
          appeal.status = upholdCount > allReviewers.length / 2 ? "upheld" : "denied";
          appeal.resolvedAt = new Date();
          await appealRepo.save(appeal);

          // If appeal is denied (succeeds), reset milestone proof to pending
          if (appeal.status === "denied") {
            const proof = await proofRepo.findOne({
              where: { grantId, milestoneIdx },
            });
            if (proof) {
              // Update MilestoneProof - set approved to null (reset to pending)
              // Note: MilestoneProof doesn't have an approved field, so we need to handle this differently
              // For now, we'll delete the rejection approvals
              await approvalRepo.delete({ grantId, milestoneIdx, approved: false });
              
              // Fire milestone_reactivated activity event
              await activityRepo.save(
                activityRepo.create({
                  type: "milestone_reactivated",
                  entityType: "milestone_proof",
                  entityId: proof.id,
                  actorAddress: appeal.appellantAddress,
                  data: { grantId, milestoneIdx, appealId: appeal.id },
                })
              );
            }
          }

          // Fire milestone_appeal_resolved webhook
          await webhookDispatcher
            .dispatch("milestone.appeal_resolved" as never, {
              appealId: appeal.id,
              grantId,
              milestoneIdx,
              status: appeal.status,
            })
            .catch(() => {});
        }

        res.json({ data: appeal });
      } catch (error) {
        next(error);
      }
    },
  );

  return router;
};
