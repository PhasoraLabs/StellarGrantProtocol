import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { GrantReviewer } from "../entities/GrantReviewer";
import { Grant } from "../entities/Grant";
import { SignatureService } from "../services/signature-service";
import { NonceService, NonceReplayError } from "../services/nonce-service";

const addReviewerSchema = z.object({
  grantId: z.number().int().positive(),
  reviewerStellarAddress: z.string().min(10).max(120),
  address: z.string().min(10).max(120),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
  signature: z.string().min(32),
});

export const buildGrantReviewerRouter = (
  reviewerRepo: Repository<GrantReviewer>,
  signatureService: SignatureService,
  grantRepo: Repository<Grant>,
  nonceService: NonceService,
) => {
  const router = Router();

  // Add a reviewer to a grant — grant owner only, signature-verified (#1238).
  router.post("/", async (req, res, next) => {
    try {
      const parsed = addReviewerSchema.safeParse(req.body);
      if (!parsed.success) {
        return res.status(400).json({ error: "Invalid payload", details: parsed.error.issues });
      }
      const payload = parsed.data;

      const maxSkewMs = 5 * 60 * 1000;
      if (Math.abs(Date.now() - payload.timestamp) > maxSkewMs) {
        return res.status(400).json({ error: "Expired intent timestamp" });
      }

      if (!signatureService.verifyAddReviewer(payload)) {
        return res.status(401).json({ error: "Invalid Stellar signature" });
      }

      await nonceService.consume(payload.nonce);

      const grant = await grantRepo.findOne({ where: { id: payload.grantId } });
      if (!grant) {
        return res.status(404).json({ error: "Grant not found" });
      }

      const owner = grant.owner || grant.recipient;
      if (owner !== payload.address) {
        return res.status(403).json({ error: "Only the grant owner can add reviewers" });
      }

      const reviewer = reviewerRepo.create({
        grantId: payload.grantId,
        reviewerStellarAddress: payload.reviewerStellarAddress,
      });
      await reviewerRepo.save(reviewer);
      return res.status(201).json({ data: reviewer });
    } catch (error: any) {
      if (error instanceof NonceReplayError) {
        return res.status(409).json({ error: "Signature nonce has already been used" });
      }
      if (error?.code === "23505" || error?.code === "SQLITE_CONSTRAINT") {
        return res.status(409).json({ error: "Reviewer already exists for this grant" });
      }
      next(error);
    }
  });

  // List reviewers for a grant
  router.get("/grant/:grantId", async (req, res) => {
    const { grantId } = req.params;
    const reviewers = await reviewerRepo.find({ where: { grantId: Number(grantId) } });
    res.json({ data: reviewers });
  });

  // Remove a reviewer from a grant
  router.delete("/:id", async (req, res) => {
    const { id } = req.params;
    await reviewerRepo.delete(id);
    res.json({ success: true });
  });

  return router;
};
