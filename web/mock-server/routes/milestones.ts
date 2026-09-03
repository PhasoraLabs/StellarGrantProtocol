import { Router } from "express";
import { milestones } from "../fixtures/milestones";

const router = Router();

router.get("/", (req, res) => {
  const q = (req.query.q as string)?.toLowerCase();
  
  if (!q) {
    return res.json({ data: milestones });
  }

  const filtered = milestones.filter(m => 
    m.title.toLowerCase().includes(q) || 
    m.description.toLowerCase().includes(q)
  );

  res.json({ data: filtered });
});

export default router;
