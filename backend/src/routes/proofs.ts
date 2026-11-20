import { Router } from "express";
import multer from "multer";
import { IpfsService } from "../services/ipfs-service";
import { buildAuthMiddleware } from "../middlewares/auth-middleware";
import { SignatureService } from "../services/signature-service";
import { NonceService } from "../services/nonce-service";
import { createWalletRateLimiter } from "../middlewares/rate-limiter";
import type { AuthenticatedRequest } from "../types/auth";

const MAX_FILE_BYTES = 50 * 1024 * 1024; // 50 MB

const upload = multer({
  storage: multer.memoryStorage(),
  limits: { fileSize: MAX_FILE_BYTES },
  fileFilter: (_req, file, cb) => {
    const ALLOWED = [
      "application/pdf",
      "image/jpeg",
      "image/png",
      "image/gif",
      "image/webp",
      "application/zip",
      "text/plain",
    ];
    if (ALLOWED.includes(file.mimetype)) {
      cb(null, true);
    } else {
      cb(new Error(`Unsupported file type: ${file.mimetype}`));
    }
  },
});

export const buildProofsRouter = (
  ipfsService: IpfsService,
  signatureService: SignatureService,
  nonceService: NonceService,
) => {
  const router = Router();
  const authMiddleware = buildAuthMiddleware(signatureService, nonceService);

  // 20 uploads per authenticated wallet per hour. Keyed off the session identity,
  // not a client-supplied field, so it cannot be rotated to bypass the limit.
  const uploadRateLimiter = createWalletRateLimiter(
    60 * 60 * 1000,
    20,
    (req) => (req as AuthenticatedRequest).user?.stellarAddress ?? null,
  );

  /**
   * POST /proofs/upload
   * Uploads a milestone proof document to IPFS and returns the CID.
   * Multipart/form-data: field name "file"
   */
  router.post(
    "/upload",
    authMiddleware,
    uploadRateLimiter,
    upload.single("file"),
    async (req, res, next) => {
      try {
        if (!req.file) {
          res.status(400).json({ error: "No file attached; use multipart field name 'file'" });
          return;
        }

        const result = await ipfsService.uploadFile(
          req.file.buffer,
          req.file.originalname,
          req.file.mimetype,
        );

        res.status(201).json({
          data: {
            cid: result.cid,
            gatewayUrl: result.gatewayUrl,
            size: result.size,
            fileName: req.file.originalname,
            mimeType: req.file.mimetype,
          },
        });
      } catch (error: unknown) {
        if (error instanceof multer.MulterError && error.code === "LIMIT_FILE_SIZE") {
          res.status(413).json({ error: "File exceeds 50 MB limit" });
          return;
        }
        next(error);
      }
    },
  );

  /**
   * GET /proofs/cid/:cid
   * Redirects to the IPFS gateway URL for the given CID.
   */
  router.get("/cid/:cid", (req, res) => {
    const { cid } = req.params;
    if (!/^[a-zA-Z0-9]+$/.test(cid)) {
      res.status(400).json({ error: "Invalid CID format" });
      return;
    }
    res.redirect(301, ipfsService.gatewayUrl(cid));
  });

  return router;
};
