import { Router } from "express";
import { Repository } from "typeorm";
import { GrantReviewer } from "../entities/GrantReviewer";

export const buildGrantReviewerRouter = (reviewerRepo: Repository<GrantReviewer>) => {
  const router = Router();

  // Add a reviewer to a grant
  router.post("/", async (req, res) => {
    const { grantId, reviewerStellarAddress } = req.body;
    if (!grantId || !reviewerStellarAddress) {
      return res.status(400).json({ error: "grantId and reviewerStellarAddress are required" });
    }
    const reviewer = reviewerRepo.create({ grantId, reviewerStellarAddress });
    await reviewerRepo.save(reviewer);
    res.json({ data: reviewer });
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
