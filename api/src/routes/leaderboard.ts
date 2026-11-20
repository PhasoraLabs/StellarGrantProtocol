import { Router } from "express";
import { Repository } from "typeorm";
import { Grant } from "../entities/Grant";

const sortFields = {
  title: "grant.title",
  status: "grant.status",
  amount: "grant.totalAmount",
  updatedAt: "grant.updatedAt",
} as const;

export const buildLeaderboardRouter = (grantRepo: Repository<Grant>) => {
  const router = Router();
  router.get("/", async (req, res, next) => {
    try {
      const sort = String(req.query.sort ?? "updatedAt") as keyof typeof sortFields;
      const direction = String(req.query.direction ?? "desc").toLowerCase();
      if (!(sort in sortFields) || !["asc", "desc"].includes(direction)) {
        res.status(400).json({ error: "sort must be title, status, amount or updatedAt; direction must be asc or desc" });
        return;
      }
      const grants = await grantRepo.createQueryBuilder("grant")
        .orderBy(sortFields[sort], direction.toUpperCase() as "ASC" | "DESC")
        .getMany();
      res.json({ data: grants, sort, direction });
    } catch (error) { next(error); }
  });
  return router;
};
