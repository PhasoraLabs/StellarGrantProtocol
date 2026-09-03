import { Router } from "express";
import { contributors } from "../fixtures/contributors";

const router = Router();

router.get("/:address", (req, res) => {
  const contributor = contributors.find(c => c.address === req.params.address);
  if (!contributor) {
    // Return empty profile for any address to allow testing UI with different addresses
    return res.json({
      data: {
        address: req.params.address,
        name: "New Stellar User",
        reputation: 0,
        grantsCompleted: 0,
        bio: "This is a mock profile for testing purposes.",
        skills: [],
        history: []
      }
    });
  }
  res.json({ data: contributor });
});

export default router;
