import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { MilestoneProof } from "../entities/MilestoneProof";
import { SignatureService } from "../services/signature-service";
import { getEmailTemplate, sendEmail } from "../services/email-service";
import { Grant } from "../entities/Grant";
import { User } from "../entities/User";
import { walletLimiters } from "../middlewares/rate-limiter";
import { encodeCursor, decodeCursor, hasCursorPageConflict } from "../utils/pagination";

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

  /**
   * @openapi
   * /milestone_proof/by-contributor/{address}:
   *   get:
   *     summary: Milestone proof history for a contributor
   *     description: >
   *       Returns a cursor-paginated list of milestone proofs submitted by a
   *       contributor, ordered by createdAt DESC. Supports both offset-based
   *       (?page=) and cursor-based (?cursor=) pagination.
   *       **?page= and ?cursor= cannot be combined** — returns 400 if both present.
   *     parameters:
   *       - in: path
   *         name: address
   *         required: true
   *         schema: { type: string }
   *       - in: query
   *         name: page
   *         schema: { type: integer, minimum: 1, default: 1 }
   *       - in: query
   *         name: limit
   *         schema: { type: integer, minimum: 1, maximum: 100, default: 20 }
   *       - in: query
   *         name: cursor
   *         schema: { type: string }
   *         description: Opaque cursor from meta.nextCursor.
   *     responses:
   *       200:
   *         description: Milestone proof history page
   *         content:
   *           application/json:
   *             schema:
   *               type: object
   *               properties:
   *                 data:
   *                   type: array
   *                 meta:
   *                   type: object
   *                   properties:
   *                     nextCursor:
   *                       type: string
   *                       nullable: true
   *                     hasMore:
   *                       type: boolean
   *                     limit:
   *                       type: integer
   *       400:
   *         description: Cannot combine ?page= and ?cursor=, or invalid cursor
   */
  router.get("/by-contributor/:address", async (req, res, next) => {
    try {
      const { address } = req.params;
      const limit = Math.min(Math.max(Number(req.query.limit) || 20, 1), 100);

      const rawCursor = req.query.cursor ? String(req.query.cursor) : undefined;
      const rawPage   = req.query.page   ? String(req.query.page)   : undefined;

      if (hasCursorPageConflict(rawPage, rawCursor)) {
        res.status(400).json({ error: "Cannot combine ?page= and ?cursor= parameters" });
        return;
      }

      const qb = proofRepo.createQueryBuilder("p")
        .where("p.submittedBy = :address", { address })
        .orderBy("p.createdAt", "DESC")
        .addOrderBy("p.id", "DESC");

      // ── Cursor-based path ────────────────────────────────────────────────
      if (rawCursor !== undefined) {
        let cursorId: number;
        let cursorTs: string;
        try {
          const decoded = decodeCursor(rawCursor);
          cursorId = decoded.id;
          cursorTs = decoded.ts;
        } catch {
          res.status(400).json({ error: "Invalid cursor" });
          return;
        }

        qb.andWhere(
          "(p.createdAt < :ts OR (p.createdAt = :ts AND p.id < :id))",
          { ts: new Date(cursorTs), id: cursorId },
        ).take(limit + 1);

        const rows = await qb.getMany();
        const hasMore = rows.length > limit;
        const page = rows.slice(0, limit);
        const last = page[page.length - 1];

        return res.json({
          data: page,
          meta: {
            nextCursor: hasMore && last ? encodeCursor(last.id, last.createdAt) : null,
            hasMore,
            limit,
          },
        });
      }

      // ── Offset-based path (backwards-compatible) ─────────────────────────
      const pageNum = Math.max(Number(rawPage) || 1, 1);
      qb.skip((pageNum - 1) * limit).take(limit);

      const [rows, total] = await qb.getManyAndCount();

      return res.json({
        data: rows,
        meta: {
          total,
          page: pageNum,
          limit,
          totalPages: Math.ceil(total / limit),
        },
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
