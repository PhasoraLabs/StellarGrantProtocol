import { Router } from "express";
import { Repository } from "typeorm";
import { createHash } from "crypto";
import { Grant } from "../entities/Grant";
import { GrantView } from "../entities/GrantView";
import { env } from "../config/env";

function hashViewer(raw: string): string {
  return createHash("sha256").update(raw).digest("hex").slice(0, 16);
}

function hourBucket(now: Date): string {
  return `${now.getUTCFullYear()}${String(now.getUTCMonth() + 1).padStart(2, "0")}${String(now.getUTCDate()).padStart(2, "0")}${String(now.getUTCHours()).padStart(2, "0")}`;
}

type Period = "day" | "week" | "month";

export const buildAnalyticsRouter = (
  grantRepo: Repository<Grant>,
  grantViewRepo: Repository<GrantView>,
) => {
  const router = Router();

  /**
   * POST /analytics/grants/:id/view
   * Records a page view for a grant. Deduplicated to one view per viewer per hour.
   */
  router.post("/grants/:id/view", async (req, res, next) => {
    try {
      const grantId = Number(req.params.id);
      if (Number.isNaN(grantId)) {
        res.status(400).json({ error: "Invalid grant id" });
        return;
      }

      const rawIdentifier =
        (req.headers["x-user-address"] as string | undefined) ??
        req.ip ??
        "anonymous";

      const viewerKey = hashViewer(rawIdentifier);
      const bucket = hourBucket(new Date());

      try {
        await grantViewRepo.insert({ grantId, viewerKey, hourBucket: bucket });
      } catch (err: any) {
        // Unique constraint violation means already counted this hour — that's fine
        if (err?.code === "23505" || err?.code === "SQLITE_CONSTRAINT") {
          res.json({ data: { recorded: false, reason: "Already counted this hour" } });
          return;
        }
        throw err;
      }

      res.status(201).json({ data: { recorded: true } });
    } catch (error) {
      next(error);
    }
  });

  /**
   * GET /analytics/grants/:id
   * Returns analytics for a grant.
   * Access restricted to grant recipient or admin addresses.
   * Query param: period=day|week|month (default: day)
   */
  router.get("/grants/:id", async (req, res, next) => {
    try {
      const grantId = Number(req.params.id);
      if (Number.isNaN(grantId)) {
        res.status(400).json({ error: "Invalid grant id" });
        return;
      }

      const grant = await grantRepo.findOne({ where: { id: grantId } });
      if (!grant) {
        res.status(404).json({ error: "Grant not found" });
        return;
      }

      // Authorization: grant recipient or admin
      const requesterAddress = req.headers["x-user-address"] as string | undefined;
      const isAdmin = requesterAddress && env.adminAddresses.includes(requesterAddress);
      const isRecipient = requesterAddress && requesterAddress === grant.recipient;

      if (!isAdmin && !isRecipient) {
        res.status(403).json({ error: "Analytics are only accessible to the grant creator or admins" });
        return;
      }

      const periodParam = String(req.query.period ?? "day") as Period;
      const validPeriods: Period[] = ["day", "week", "month"];
      const period: Period = validPeriods.includes(periodParam) ? periodParam : "day";

      const daysBack = period === "day" ? 1 : period === "week" ? 7 : 30;
      const since = new Date(Date.now() - daysBack * 24 * 60 * 60 * 1000);

      const views = await grantViewRepo
        .createQueryBuilder("gv")
        .where("gv.grantId = :grantId", { grantId })
        .andWhere("gv.createdAt >= :since", { since })
        .getMany();

      // Aggregate by UTC date
      const byDate: Record<string, number> = {};
      for (const v of views) {
        const day = v.createdAt.toISOString().slice(0, 10);
        byDate[day] = (byDate[day] ?? 0) + 1;
      }

      const breakdown = Object.entries(byDate)
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([date, count]) => ({ date, count }));

      res.json({
        data: {
          grantId,
          period,
          totalViews: views.length,
          uniqueViewers: new Set(views.map(v => v.viewerKey)).size,
          breakdown,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
