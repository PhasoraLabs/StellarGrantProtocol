/**
 * POST /ipfs/presign   — generate a short-lived Pinata upload token.
 * POST /ipfs/verify    — confirm a CID is publicly accessible.
 *
 * Issue #453: Presigned IPFS Upload URLs — Bypass API for Large Files.
 */

import { Router, Response } from "express";
import { z } from "zod";
import { authMiddleware } from "../middlewares/auth-middleware";
import { createWalletRateLimiter } from "../middlewares/rate-limiter";
import { ipfsService } from "../services/ipfs-service";
import type { AuthenticatedRequest } from "../types/auth";

const presignBody = z.object({
  address: z.string().min(10).max(120),
});

const verifyBody = z.object({
  cid: z.string().min(10).max(100),
});

// 10 presign requests per wallet per hour — see issue spec (rate-limit 10/hr)
const presignRateLimiter = createWalletRateLimiter(60 * 60 * 1000, 10);

export function buildIpfsPresignRouter(): Router {
  const router = Router();

  /**
   * POST /ipfs/presign
   *
   * Returns a single-use Pinata JWT the frontend can use to upload directly,
   * bypassing this server for large file transfers.
   *
   * Requires: connected wallet (x-stellar-address header via authMiddleware).
   * Rate-limited: 10 requests per wallet per hour.
   */
  router.post(
    "/presign",
    authMiddleware as any,
    presignRateLimiter,
    async (req: any, res: Response) => {
      try {
        const body = presignBody.safeParse(req.body);
        if (!body.success) {
          res.status(400).json({ error: "address is required" });
          return;
        }

        const data = await ipfsService.generatePresignedKey(body.data.address);
        res.status(200).json({ data });
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        res.status(500).json({ error: msg });
      }
    },
  );

  /**
   * POST /ipfs/verify
   *
   * Confirms that a CID is publicly accessible on IPFS. Call this after a
   * direct-to-Pinata upload to validate the CID before storing it on-chain.
   */
  router.post(
    "/verify",
    authMiddleware as any,
    async (req: any, res: Response) => {
      try {
        const body = verifyBody.safeParse(req.body);
        if (!body.success) {
          res.status(400).json({ error: "cid is required" });
          return;
        }

        const accessible = await ipfsService.verifyCid(body.data.cid);
        res.status(200).json({ data: { cid: body.data.cid, accessible } });
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        res.status(500).json({ error: msg });
      }
    },
  );

  return router;
}
