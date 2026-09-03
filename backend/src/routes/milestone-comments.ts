import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { Milestone } from "../entities/Milestone";
import { MilestoneComment } from "../entities/MilestoneComment";
import { GrantReviewer } from "../entities/GrantReviewer";
import { env } from "../config/env";
import { notificationService } from "../services/notification-service";
import { validateParams, validateRequest } from "../middlewares/validation-middleware";
import { milestoneCommentCreateSchema, idParamSchema } from "../schemas";

const isAdmin = (address?: string) => !!address && env.adminAddresses.includes(address);

export const buildMilestoneCommentsRouter = (
  milestoneRepo: Repository<Milestone>,
  commentsRepo: Repository<MilestoneComment>,
  reviewerRepo: Repository<GrantReviewer>,
) => {
  const router = Router();

  router.get("/milestones/:id/comments", validateParams(idParamSchema), async (req, res, next) => {
    try {
      const { id } = (req as any).validatedParams;

      const milestone = await milestoneRepo.findOne({ where: { id } });
      if (!milestone) {
        res.status(404).json({ error: "Milestone not found" });
        return;
      }

      const comments = await commentsRepo.find({
        where: { milestoneId: id },
        order: { createdAt: "ASC", id: "ASC" },
      });

      res.json({ data: comments });
    } catch (error) {
      next(error);
    }
  });

  router.post("/milestones/:id/comments", validateRequest({ params: idParamSchema, body: milestoneCommentCreateSchema }), async (req, res, next) => {
    try {
      const { id } = (req as any).validatedParams;
      const payload = (req as any).validatedBody;

      const milestone = await milestoneRepo.findOne({
        where: { id },
        relations: { grant: true },
      });
      if (!milestone) {
        res.status(404).json({ error: "Milestone not found" });
        return;
      }

      if (payload.parentCommentId) {
        const parent = await commentsRepo.findOne({ where: { id: payload.parentCommentId } });
        if (!parent || parent.milestoneId !== id) {
          res.status(400).json({ error: "Invalid parentCommentId" });
          return;
        }
      }

      const saved = await commentsRepo.save({
        milestoneId: id,
        content: payload.content,
        authorAddress: payload.authorAddress,
        parentCommentId: payload.parentCommentId ?? null,
      });

      const reviewers = await reviewerRepo.find({ where: { grantId: milestone.grantId } });
      const recipients = new Set<string>([milestone.grant.recipient, ...reviewers.map((reviewer) => reviewer.reviewerStellarAddress)]);
      recipients.delete(payload.authorAddress);

      for (const address of recipients) {
        notificationService.notifyUser(address, "milestone_comment_added", {
          commentId: saved.id,
          milestoneId: id,
          grantId: milestone.grantId,
          authorAddress: payload.authorAddress,
        });
      }

      res.status(201).json({ data: saved });
    } catch (error) {
      next(error);
    }
  });

  router.delete("/milestones/:id/comments/:commentId", validateRequest({ params: z.object({ id: z.coerce.number().int().positive(), commentId: z.coerce.number().int().positive() }) }), async (req, res, next) => {
    try {
      const { id, commentId } = (req as any).validatedParams;

      const actor = req.header("x-admin-address") ?? undefined;
      if (!isAdmin(actor)) {
        res.status(403).json({ error: "Admin privileges required" });
        return;
      }

      const comment = await commentsRepo.findOne({ where: { id: commentId, milestoneId: id } });
      if (!comment) {
        res.status(404).json({ error: "Comment not found" });
        return;
      }

      await commentsRepo.delete({ id: comment.id });
      res.json({ ok: true });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
