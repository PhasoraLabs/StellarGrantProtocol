import { Router } from "express";
import { grants } from "../fixtures/grants";

const router = Router();

router.get("/:address", (req, res) => {
  // Mock deadlines for a specific address
  const deadlines = grants
    .filter(g => g.status === 'active')
    .slice(0, 3)
    .map(g => ({
      grantId: g.id,
      grantTitle: g.title,
      deadline: g.deadline,
      daysRemaining: Math.floor(Math.random() * 30) + 1
    }));

  res.json({ data: { deadlines } });
});

export default router;
