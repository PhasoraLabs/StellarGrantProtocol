import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { Contributor } from "../entities/Contributor";

const prefsSchema = z.object({
  address: z.string().min(10).max(120),
  email: z.string().email().max(254).optional(),
  emailNotifications: z.boolean(),
});

export const buildNotificationsRouter = (contributorRepo: Repository<Contributor>) => {
  const router = Router();

  /**
   * PUT /notifications/preferences
   * Opt-in or opt-out of email notifications for a given Stellar address.
   */
  router.put("/preferences", async (req, res, next) => {
    try {
      const parsed = prefsSchema.safeParse(req.body);
      if (!parsed.success) {
        res.status(400).json({ error: "Invalid payload", details: parsed.error.issues });
        return;
      }

      const { address, email, emailNotifications } = parsed.data;

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
   */
  router.get("/preferences/:address", async (req, res, next) => {
    try {
      const { address } = req.params;
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
