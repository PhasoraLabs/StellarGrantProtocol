import { Router } from "express";
import { Repository, In } from "typeorm";
import { UserWatchlist } from "../entities/UserWatchlist";
import { Grant } from "../entities/Grant";
import { Activity } from "../entities/Activity";
import { validateParams } from "../middlewares/validation-middleware";
import { idParamSchema } from "../schemas";

export const buildWatchlistRouter = (
  watchlistRepo: Repository<UserWatchlist>,
  grantRepo: Repository<Grant>,
) => {
  const router = Router();
  const activityRepo = watchlistRepo.manager.getRepository(Activity);

  router.get("/", async (req, res, next) => {
    try {
      const address = req.header("x-user-address");
      if (!address) {
        res.status(400).json({ error: "Missing x-user-address header" });
        return;
      }

      const watchlist = await watchlistRepo.find({ where: { address } });
      const grantIds = watchlist.map((w) => w.grantId);

      if (grantIds.length === 0) {
        res.json({ data: [] });
        return;
      }

      const grants = await grantRepo.find({
        where: { id: In(grantIds) },
      });
      res.json({ data: grants });
    } catch (error) {
      next(error);
    }
  });

  router.post("/:grantId", validateParams(idParamSchema), async (req, res, next) => {
    try {
      const { id } = (req as any).validatedParams;
      const address = req.header("x-user-address");

      if (!address) {
        res.status(400).json({ error: "Missing x-user-address header" });
        return;
      }

      const grant = await grantRepo.findOne({ where: { id } });
      if (!grant) {
        res.status(404).json({ error: "Grant not found" });
        return;
      }

      await watchlistRepo.save({ address, grantId: id });

      await activityRepo.save({
        type: "watchlist_added" as any,
        entityType: "grant",
        entityId: id,
        actorAddress: address,
        data: null,
      });

      res.status(201).json({ ok: true });
    } catch (error: any) {
      if (error?.code === "23505" || error?.code === "SQLITE_CONSTRAINT") {
        res.status(409).json({ error: "Grant already in watchlist" });
        return;
      }
      next(error);
    }
  });

  router.delete("/:grantId", validateParams(idParamSchema), async (req, res, next) => {
    try {
      const { id } = (req as any).validatedParams;
      const address = req.header("x-user-address");

      if (!address) {
        res.status(400).json({ error: "Missing x-user-address header" });
        return;
      }

      const result = await watchlistRepo.delete({ address, grantId: id });

      if (result.affected === 0) {
        res.status(404).json({ error: "Watchlist entry not found" });
        return;
      }

      await activityRepo.save({
        type: "watchlist_removed" as any,
        entityType: "grant",
        entityId: id,
        actorAddress: address,
        data: null,
      });

      res.json({ ok: true });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
