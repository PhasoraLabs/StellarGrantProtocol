import { createHash } from "node:crypto";
import { Request, Response, NextFunction } from "express";
import { SignatureService } from "../services/signature-service";
import { NonceReplayError, NonceService } from "../services/nonce-service";
import { AuthenticatedRequest } from "../types/auth";
import { UnauthorizedError } from "../utils/errors";

const MAX_SKEW_MS = 5 * 60 * 1000;

/**
 * Establishes `req.user.stellarAddress` only from a verified signature.
 *
 * Headers: x-stellar-address, x-stellar-signature (base64), x-stellar-nonce,
 * x-stellar-timestamp. The signed action is `METHOD:/path` (no query string)
 * and each nonce is accepted once.
 *
 * With `optional: true` requests without credentials pass through with no
 * `req.user`, so public reads keep working while RBAC checks fail closed.
 * Present-but-invalid credentials are always rejected.
 */
export const buildSignedUserAuth = (
  signatureService: SignatureService,
  nonceService: NonceService,
  options: { optional?: boolean } = {},
) => {
  return async (req: Request, res: Response, next: NextFunction) => {
    const address = req.header("x-stellar-address");
    const signature = req.header("x-stellar-signature");
    const nonce = req.header("x-stellar-nonce");
    const timestamp = parseInt(req.header("x-stellar-timestamp") ?? "0", 10);

    if (!address && !signature && !nonce && !req.header("x-stellar-timestamp")) {
      if (options.optional) return next();
      next(new UnauthorizedError("Signed authentication required"));
      return;
    }

    if (!address || !signature || !nonce || !timestamp) {
      next(new UnauthorizedError("Missing authentication headers"));
      return;
    }

    if (Math.abs(Date.now() - timestamp) > MAX_SKEW_MS) {
      next(new UnauthorizedError("Expired authentication timestamp"));
      return;
    }

    const action = `${req.method}:${req.originalUrl.split("?")[0]}`;
    if (!signatureService.verifyUser({ address, nonce, timestamp, action, signature })) {
      next(new UnauthorizedError("Invalid authentication signature"));
      return;
    }

    try {
      const scoped = createHash("sha256").update(`${address}:${nonce}`).digest("hex");
      await nonceService.consume(`user:${scoped}`);
    } catch (err) {
      if (err instanceof NonceReplayError) {
        next(new UnauthorizedError("Authentication nonce has already been used"));
        return;
      }
      next(err);
      return;
    }

    (req as AuthenticatedRequest).user = { stellarAddress: address };
    next();
  };
};
