import { Router } from "express";
import { leaderboard } from "../fixtures/leaderboard";

const router = Router();

router.get("/", (req, res) => {
  const page = parseInt(req.query.page as string) || 1;
  const limit = parseInt(req.query.limit as string) || 10;

  const startIndex = (page - 1) * limit;
  const paginated = leaderboard.slice(startIndex, startIndex + limit);

  res.json({
    data: paginated,
    total: leaderboard.length,
    nextPage: startIndex + limit < leaderboard.length ? page + 1 : null
  });
});

export default router;
