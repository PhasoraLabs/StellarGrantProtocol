import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { Contributor } from "../entities/Contributor";
import { buildAuthMiddleware } from "../middlewares/auth-middleware";
import { SignatureService } from "../services/signature-service";
import { NonceService } from "../services/nonce-service";
import type { AuthenticatedRequest } from "../types/auth";
import type { Request as ExpressRequest } from "express";

const prefsSchema = z.object({
  address: z.string().min(10).max(120),
  email: z.string().email().max(254).optional(),
  emailNotifications: z.boolean(),
});

export const buildNotificationsRouter = (
  contributorRepo: Repository<Contributor>,
  signatureService: SignatureService,
  nonceService: NonceService,
) => {
  const router = Router();
  const authMiddleware = buildAuthMiddleware(signatureService, nonceService);

  /**
   * PUT /notifications/preferences
   * Opt-in or opt-out of email notifications for a given Stellar address.
   */
  router.put("/preferences", authMiddleware, async (req: any, res, next) => {
    try {
      const parsed = prefsSchema.safeParse(req.body);
      if (!parsed.success) {
        res.status(400).json({ error: "Invalid payload", details: parsed.error.issues });
        return;
      }

      const { address, email, emailNotifications } = parsed.data;

      // Auth: only the address owner can update preferences
      if (req.user.stellarAddress !== address) {
        res.status(403).json({ error: "Not authorized to update preferences for this address" });
        return;
      }

      let contributor = await contributorRepo.findOne({ where: { address } });
      if (!contributor) {
        contributor = contributorRepo.create({ address });
      }

      contributor.emailNotifications = emailNotifications;
      if (email !== undefined) {
        contributor.email = email;
      }

      await contributorRepo.save(contributor);
      res.json({ data: { address, emailNotifications, email: contributor.email } });
    } catch (error) {
      next(error);
    }
  });

  /**
   * GET /notifications/preferences/:address
   * Returns the current notification preferences for an address.
   * Requires authentication and proof of address ownership.
   */
  router.get("/preferences/:address", authMiddleware, async (req: any, res, next) => {
    try {
      const { address } = req.params;

      // Auth: only the address owner can read preferences
      if (req.user.stellarAddress !== address) {
        res.status(403).json({ error: "Not authorized to read preferences for this address" });
        return;
      }

      const contributor = await contributorRepo.findOne({ where: { address } });

      res.json({
        data: {
          address,
          emailNotifications: contributor?.emailNotifications ?? true,
          email: contributor?.email ?? null,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
