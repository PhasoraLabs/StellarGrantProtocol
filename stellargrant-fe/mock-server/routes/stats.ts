import { Router } from "express";
import { stats } from "../fixtures/stats";

const router = Router();

router.get("/", (req, res) => {
  res.json({ data: stats });
});

export default router;
