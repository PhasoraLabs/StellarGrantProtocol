import { Router } from "express";
import { Repository } from "typeorm";
import { User } from "../entities/User";
import { validateBody, validateParams } from "../middlewares/validation-middleware";
import { userRegisterSchema, addressParamSchema } from "../schemas";

export const buildUserRouter = (userRepo: Repository<User>) => {
  const router = Router();

  // Register or update user email and notification preferences
  router.post("/register", validateBody(userRegisterSchema), async (req, res) => {
    const { email, stellarAddress, notifyMilestoneApproved, notifyMilestoneSubmitted } = req.body;
    let user = await userRepo.findOne({ where: { stellarAddress } });
    if (user) {
      user.email = email;
      if (notifyMilestoneApproved !== undefined) user.notifyMilestoneApproved = notifyMilestoneApproved;
      if (notifyMilestoneSubmitted !== undefined) user.notifyMilestoneSubmitted = notifyMilestoneSubmitted;
    } else {
      user = userRepo.create({ email, stellarAddress, notifyMilestoneApproved, notifyMilestoneSubmitted });
    }
    await userRepo.save(user);
    res.json({ data: user });
  });

  // Get user notification preferences
  router.get("/:stellarAddress", validateParams(addressParamSchema), async (req, res) => {
    const { address } = (req as any).validatedParams;
    const user = await userRepo.findOne({ where: { stellarAddress: address } });
    if (!user) return res.status(404).json({ error: "User not found" });
    res.json({ data: user });
  });

  return router;
};
