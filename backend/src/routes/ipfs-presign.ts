/**
 * POST /ipfs/presign   — generate a short-lived Pinata upload token.
 * POST /ipfs/verify    — confirm a CID is publicly accessible.
 *
 * Issue #453: Presigned IPFS Upload URLs — Bypass API for Large Files.
 * Issue #1430: Max file size validation for presign requests.
 */

import { Router, Response } from "express";
import { z } from "zod";
import { buildAuthMiddleware } from "../middlewares/auth-middleware";
import { SignatureService } from "../services/signature-service";
import { NonceService } from "../services/nonce-service";
import { createWalletRateLimiter } from "../middlewares/rate-limiter";
import { ipfsService } from "../services/ipfs-service";
import type { AuthenticatedRequest } from "../types/auth";

export const DEFAULT_MAX_FILE_SIZE = 10 * 1024 * 1024; // 10 MB default

export interface IpfsPresignOptions {
  maxFileSize?: number;
  ipfsServiceInstance?: typeof ipfsService;
  authMiddleware?: (req: any, res: any, next: any) => void;
}

const presignBodySchema = z
  .object({
    fileSize: z
      .number({ invalid_type_error: "fileSize must be a number" })
      .int("fileSize must be an integer")
      .positive("fileSize must be a positive number")
      .optional(),
    size: z
      .number({ invalid_type_error: "size must be a number" })
      .int("size must be an integer")
      .positive("size must be a positive number")
      .optional(),
  })
  .passthrough();

const verifyBody = z.object({
  cid: z.string().min(10).max(100),
});

export function buildIpfsPresignRouter(
  signatureService: SignatureService,
  nonceService: NonceService,
  options?: IpfsPresignOptions,
): Router {
  const router = Router();
  const authMiddleware =
    options?.authMiddleware ?? buildAuthMiddleware(signatureService, nonceService);

  const configuredMaxFileSize =
    options?.maxFileSize ??
    (process.env.IPFS_MAX_FILE_SIZE_BYTES
      ? Number(process.env.IPFS_MAX_FILE_SIZE_BYTES)
      : DEFAULT_MAX_FILE_SIZE);

  const service = options?.ipfsServiceInstance ?? ipfsService;

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
   * Validates declared file size against the configured maximum limit.
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

        // Validate declared file size in body if present
        const parsedBody = presignBodySchema.safeParse(req.body ?? {});
        if (!parsedBody.success) {
          res.status(400).json({
            error:
              parsedBody.error.errors[0]?.message ?? "Invalid file size parameter",
          });
          return;
        }

        const querySize = req.query?.fileSize
          ? Number(req.query.fileSize)
          : req.query?.size
            ? Number(req.query.size)
            : undefined;

        if (
          querySize !== undefined &&
          (!Number.isFinite(querySize) || querySize <= 0)
        ) {
          res.status(400).json({ error: "Query file size must be a positive number" });
          return;
        }

        const declaredSize =
          parsedBody.data.fileSize ?? parsedBody.data.size ?? querySize;

        if (declaredSize !== undefined && declaredSize > configuredMaxFileSize) {
          res.status(400).json({
            error: `File size (${declaredSize} bytes) exceeds maximum allowed size of ${configuredMaxFileSize} bytes`,
            maxSize: configuredMaxFileSize,
          });
          return;
        }

        const data = await service.generatePresignedKey(address);
        res.status(200).json({
          data: {
            ...data,
            maxSize: configuredMaxFileSize,
          },
        });
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

        const accessible = await service.verifyCid(body.data.cid);
        res.status(200).json({ data: { cid: body.data.cid, accessible } });
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        res.status(500).json({ error: msg });
      }
    },
  );

  return router;
}
