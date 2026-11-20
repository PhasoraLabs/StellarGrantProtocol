/**
 * POST /ipfs/presign   — generate a short-lived Pinata upload token.
 * POST /ipfs/verify    — confirm a CID is publicly accessible.
 *
 * Issue #453: Presigned IPFS Upload URLs — Bypass API for Large Files.
 */

import { Router, Response } from "express";
import { z } from "zod";
import { buildAuthMiddleware } from "../middlewares/auth-middleware";
import { SignatureService } from "../services/signature-service";
import { NonceService } from "../services/nonce-service";
import { createWalletRateLimiter } from "../middlewares/rate-limiter";
import { ipfsService } from "../services/ipfs-service";
import type { AuthenticatedRequest } from "../types/auth";

const verifyBody = z.object({
  cid: z.string().min(10).max(100),
});

export function buildIpfsPresignRouter(
  signatureService: SignatureService,
  nonceService: NonceService,
): Router {
  const router = Router();
  const authMiddleware = buildAuthMiddleware(signatureService, nonceService);

  // 10 presign requests per authenticated wallet per hour — see issue spec (rate-limit 10/hr).
  // The key is derived from the authenticated session identity (set by authMiddleware),
  // never from a client-supplied body field, so it cannot be spoofed.
  const presignRateLimiter = createWalletRateLimiter(
    60 * 60 * 1000,
    10,
    (req) => (req as AuthenticatedRequest).user?.stellarAddress ?? null,
  );

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
    authMiddleware,
    presignRateLimiter,
    async (req: AuthenticatedRequest, res: Response) => {
      try {
        const address = req.user?.stellarAddress;
        if (!address) {
          res.status(401).json({ error: "Authentication required" });
          return;
        }

        const data = await ipfsService.generatePresignedKey(address);
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
    authMiddleware,
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
