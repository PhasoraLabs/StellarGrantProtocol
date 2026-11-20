import { Response, NextFunction } from "express";
import { AuthenticatedRequest } from "../types/auth";
import { UnauthorizedError } from "../utils/errors";
import { SignatureService } from "../services/signature-service";
import { NonceReplayError, NonceService } from "../services/nonce-service";

const MAX_SKEW_MS = 5 * 60 * 1000;

/**
 * Establishes `req.user.stellarAddress` only from a verified signature.
 *
 * Headers: x-stellar-address, x-stellar-signature (base64), x-stellar-nonce,
 * x-stellar-timestamp. The signed action is `METHOD:/path` (no query string)
 * and each nonce is accepted once.
 *
 * Issue #1194: Previously trusted the unverified x-stellar-address header,
 * allowing anyone to impersonate any Stellar address. Now requires
 * cryptographic proof of ownership via Ed25519 signature verification.
 */
export function buildAuthMiddleware(
  signatureService: SignatureService,
  nonceService: NonceService,
) {
  return async (req: AuthenticatedRequest, _res: Response, next: NextFunction) => {
    const address = req.header("x-stellar-address");
    const signature = req.header("x-stellar-signature");
    const nonce = req.header("x-stellar-nonce");
    const timestamp = parseInt(req.header("x-stellar-timestamp") ?? "0", 10);

    if (!address || !signature || !nonce || !timestamp) {
      return next(new UnauthorizedError("Missing authentication headers"));
    }

    if (Math.abs(Date.now() - timestamp) > MAX_SKEW_MS) {
      return next(new UnauthorizedError("Expired authentication timestamp"));
    }

    const action = `${req.method}:${req.originalUrl.split("?")[0]}`;
    if (!signatureService.verifyUser({ address, nonce, timestamp, action, signature })) {
      return next(new UnauthorizedError("Invalid authentication signature"));
    }

    try {
      const scoped = `${address}:${nonce}`;
      await nonceService.consume(`user:${scoped}`);
    } catch (err) {
      if (err instanceof NonceReplayError) {
        return next(new UnauthorizedError("Authentication nonce has already been used"));
      }
      return next(err);
    }

    req.user = { stellarAddress: address };
    next();
  };
}
