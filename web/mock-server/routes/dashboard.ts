import { Router } from "express";
import { grants } from "../fixtures/grants";

const router = Router();

router.get("/:address", (req, res) => {
  // Mock deadlines for a specific address
  const deadlines = grants
    .filter((g) => g.status === 1) // 1 = Active
    .slice(0, 3)
    .map((g) => ({
      grantId: g.id,
      grantTitle: g.title,
      deadline: new Date(Number(g.deadline) * 1000).toISOString().slice(0, 10),
      daysRemaining: Math.floor(Math.random() * 30) + 1,
    }));

  res.json({ data: { deadlines } });
});

export default router;
