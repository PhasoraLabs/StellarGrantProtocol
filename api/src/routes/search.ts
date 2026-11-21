import { Router } from "express";
import { Repository } from "typeorm";
import { Grant } from "../entities/Grant";

export const buildSearchRouter = (grantRepo: Repository<Grant>) => {
  const router = Router();
  router.get("/", async (req, res, next) => {
    try {
      const query = String(req.query.q ?? "").trim();
      if (query.length < 2 || query.length > 120) {
        res.status(400).json({ error: "q must contain between 2 and 120 characters" });
        return;
      }
      const grants = await grantRepo.createQueryBuilder("grant")
        .where("LOWER(grant.title) LIKE LOWER(:query) OR LOWER(grant.description) LIKE LOWER(:query)", { query: `%${query}%` })
        .orderBy("grant.updatedAt", "DESC")
        .take(50)
        .getMany();
      res.json({ data: grants, query });
    } catch (error) { next(error); }
  });
  return router;
};
