import { Router } from "express";
import { Repository } from "typeorm";
import { Activity } from "../entities/Activity";
import { Contributor } from "../entities/Contributor";
import { encodeCursor, decodeCursor } from "../utils/pagination";

export const buildActivityRouter = (
  activityRepo: Repository<Activity>,
  contributorRepo: Repository<Contributor>,
) => {
  const router = Router();

  /**
   * @openapi
   * /activity:
   *   get:
   *     summary: Activity feed
   *     description: >
   *       Returns a cursor-paginated activity feed ordered by timestamp DESC.
   *       Pass `?cursor=` from a previous response's `meta.nextCursor` to fetch
   *       the next page. The legacy `?after_id=` param is also accepted as an
   *       alias for `?cursor=` for backwards compatibility with SSE polling.
   *     parameters:
   *       - in: query
   *         name: limit
   *         schema: { type: integer, minimum: 1, maximum: 100, default: 20 }
   *       - in: query
   *         name: cursor
   *         schema: { type: string }
   *         description: >
   *           Opaque cursor from meta.nextCursor. When provided, returns
   *           activities older than the cursor position.
   *       - in: query
   *         name: after_id
   *         schema: { type: integer }
   *         description: >
   *           Legacy SSE polling param. Returns activities with id > after_id.
   *           Ignored when ?cursor= is present.
   *     responses:
   *       200:
   *         description: Activity feed page
   *         content:
   *           application/json:
   *             schema:
   *               type: object
   *               properties:
   *                 data:
   *                   type: array
   *                 meta:
   *                   type: object
   *                   properties:
   *                     nextCursor:
   *                       type: string
   *                       nullable: true
   *                       description: Cursor for the next (older) page. null when exhausted.
   *                     hasMore:
   *                       type: boolean
   *                     limit:
   *                       type: integer
   *       400:
   *         description: Invalid cursor
   */
  router.get("/", async (req, res, next) => {
    try {
      const limit = Math.min(Number(req.query.limit) || 20, 100);
      const rawCursor = req.query.cursor ? String(req.query.cursor) : null;
      // Legacy SSE polling alias
      const afterId = req.query.after_id ? Number(req.query.after_id) : null;

      const qb = activityRepo.createQueryBuilder("activity")
        .orderBy("activity.timestamp", "DESC")
        .addOrderBy("activity.id", "DESC")
        .limit(limit + 1); // fetch one extra to detect hasMore

      if (rawCursor) {
        // Structured cursor: (timestamp, id) keyset
        let cursorId: number;
        let cursorTs: string;
        try {
          const decoded = decodeCursor(rawCursor);
          cursorId = decoded.id;
          cursorTs = decoded.ts;
        } catch {
          res.status(400).json({ error: "Invalid cursor" });
          return;
        }

        qb.andWhere(
          "(activity.timestamp < :ts OR (activity.timestamp = :ts AND activity.id < :id))",
          { ts: new Date(cursorTs), id: cursorId },
        );
      } else if (afterId !== null && !Number.isNaN(afterId)) {
        // Legacy SSE polling: return activities newer than after_id
        // (ascending order for SSE, different from the default DESC feed)
        qb.andWhere("activity.id > :afterId", { afterId })
          .orderBy("activity.id", "ASC")
          .limit(limit + 1);
      }

      const activities = await qb.getMany();

      // Enrich with contributor profiles
      const addresses = [
        ...new Set(activities.map((a) => a.actorAddress).filter(Boolean)),
      ] as string[];
      const contributors = addresses.length
        ? await contributorRepo.findBy(addresses.map((address) => ({ address })))
        : [];
      const byAddress = new Map(contributors.map((c) => [c.address, c]));

      // Determine next cursor
      let nextCursor: string | null = null;
      let hasMore = false;

      if (activities.length > limit) {
        hasMore = true;
        activities.pop(); // remove the extra probe item
        const last = activities[activities.length - 1];
        if (last) {
          nextCursor = encodeCursor(last.id, last.timestamp);
        }
      }

      res.json({
        data: activities.map((a) => ({
          ...a,
          actorProfile: a.actorAddress
            ? {
                address: a.actorAddress,
                bio: byAddress.get(a.actorAddress)?.bio ?? null,
                profilePictureUrl: byAddress.get(a.actorAddress)?.profilePictureUrl ?? null,
                githubUrl: byAddress.get(a.actorAddress)?.githubUrl ?? null,
                twitterUrl: byAddress.get(a.actorAddress)?.twitterUrl ?? null,
                linkedinUrl: byAddress.get(a.actorAddress)?.linkedinUrl ?? null,
                updatedAt: byAddress.get(a.actorAddress)?.updatedAt ?? null,
              }
            : null,
        })),
        meta: {
          nextCursor,
          hasMore,
          limit,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
