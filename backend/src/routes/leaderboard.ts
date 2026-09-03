import { Router } from "express";
import { LeaderboardService } from "../services/leaderboard-service";
import { hasCursorPageConflict } from "../utils/pagination";

/**
 * Encode a leaderboard cursor from a contributor address.
 * Uses base64url so the value is URL-safe without percent-encoding.
 */
function encodeLeaderboardCursor(address: string): string {
  return Buffer.from(JSON.stringify({ address })).toString("base64url");
}

/**
 * Decode a leaderboard cursor back to an address.
 * Throws if the cursor is malformed.
 */
function decodeLeaderboardCursor(cursor: string): string {
  try {
    const raw = Buffer.from(cursor, "base64url").toString("utf8");
    const parsed = JSON.parse(raw) as unknown;
    if (
      typeof parsed !== "object" ||
      parsed === null ||
      typeof (parsed as Record<string, unknown>).address !== "string"
    ) {
      throw new Error("Invalid cursor shape");
    }
    return (parsed as { address: string }).address;
  } catch {
    throw new Error("Invalid or malformed cursor");
  }
}

export const buildLeaderboardRouter = (leaderboardService: LeaderboardService) => {
  const router = Router();

  /**
   * @openapi
   * /leaderboard:
   *   get:
   *     summary: Contributor leaderboard
   *     description: >
   *       Returns a paginated leaderboard. Supports both offset-based
   *       pagination (?page=) and cursor-based pagination (?cursor=).
   *       **?page= and ?cursor= cannot be combined** — returns 400 if both present.
   *     parameters:
   *       - in: query
   *         name: period
   *         schema: { type: string, enum: [all-time, monthly], default: all-time }
   *       - in: query
   *         name: page
   *         schema: { type: integer, minimum: 1, default: 1 }
   *         description: Offset page number. Ignored when cursor is provided.
   *       - in: query
   *         name: limit
   *         schema: { type: integer, minimum: 1, maximum: 100, default: 20 }
   *       - in: query
   *         name: cursor
   *         schema: { type: string }
   *         description: >
   *           Opaque cursor from meta.nextCursor. Encodes the last-seen
   *           contributor address for keyset pagination.
   *     responses:
   *       200:
   *         description: Leaderboard page
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
   *                       description: Cursor for the next page. null when exhausted.
   *                     hasMore:
   *                       type: boolean
   *                     total:
   *                       type: integer
   *                     page:
   *                       type: integer
   *                       description: Only present for offset pagination.
   *                     limit:
   *                       type: integer
   *       400:
   *         description: Cannot combine ?page= and ?cursor=, or invalid cursor
   */
  router.get("/", async (req, res, next) => {
    try {
      const period = req.query.period === "monthly" ? "monthly" : "all-time";
      const limit  = Math.min(Math.max(Number(req.query.limit) || 20, 1), 100);

      const rawCursor = req.query.cursor ? String(req.query.cursor) : undefined;
      const rawPage   = req.query.page   ? String(req.query.page)   : undefined;

      // Reject combined usage
      if (hasCursorPageConflict(rawPage, rawCursor)) {
        res.status(400).json({ error: "Cannot combine ?page= and ?cursor= parameters" });
        return;
      }

      // ── Cursor-based path ──────────────────────────────────────────────────
      if (rawCursor !== undefined) {
        let afterAddress: string;
        try {
          afterAddress = decodeLeaderboardCursor(rawCursor);
        } catch {
          res.status(400).json({ error: "Invalid cursor" });
          return;
        }

        const [data, total] = await leaderboardService.getLeaderboardAfterCursor(
          period,
          afterAddress,
          limit,
        );

        const hasMore = data.length > limit;
        const page = data.slice(0, limit);
        const last = page[page.length - 1];

        return res.json({
          data: page,
          meta: {
            nextCursor: hasMore && last ? encodeLeaderboardCursor(last.address) : null,
            hasMore,
            total,
            limit,
          },
        });
      }

      // ── Offset-based path (backwards-compatible) ───────────────────────────
      const page = Math.max(Number(rawPage) || 1, 1);
      const [data, total] = await leaderboardService.getLeaderboard(period, page, limit);

      return res.json({
        data,
        meta: {
          total,
          page,
          limit,
          totalPages: Math.ceil(Number(total) / limit),
        },
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
