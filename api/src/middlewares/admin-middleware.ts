import { Request, Response, NextFunction } from "express";
import { env } from "../config/env";
import { SignatureService } from "../services/signature-service";

export const buildAdminMiddleware = (signatureService: SignatureService) => {
  return (req: Request, res: Response, next: NextFunction) => {
    const address = req.header("x-admin-address");
    const signature = req.header("x-admin-signature");
    const nonce = req.header("x-admin-nonce");
    const timestamp = parseInt(req.header("x-admin-timestamp") ?? "0", 10);

    if (!address || !signature || !nonce || !timestamp) {
      res.status(401).json({ error: "Missing admin authentication headers" });
      return;
    }

    if (!env.adminAddresses.includes(address)) {
      res.status(403).json({ error: "Unauthorized admin address" });
      return;
    }

    const maxSkewMs = 5 * 60 * 1000;
    if (Math.abs(Date.now() - timestamp) > maxSkewMs) {
      res.status(401).json({ error: "Expired admin intent timestamp" });
      return;
    }

    const action = `${req.method}:${req.originalUrl.split("?")[0]}`;
    const isValid = signatureService.verifyAdmin({
      address,
      nonce,
      timestamp,
      action,
      signature,
    });

    if (!isValid) {
      res.status(401).json({ error: "Invalid admin signature" });
      return;
    }

    // Attach admin address to request for logging
    (req as any).adminAddress = address;
    next();
  };
};
