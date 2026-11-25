import { Request, Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { User } from "../entities/User";
import { validateBody, validateParams, validateQuery } from "../middlewares/validation-middleware";
import { userRegisterSchema, addressParamSchema } from "../schemas";
import {
  DEFAULT_LIMIT,
  MAX_LIMIT,
  MAX_PAGE,
  MIN_LIMIT,
  calculatePaginationMeta,
  getPaginationOffset,
  normalizePagination,
} from "../utils/pagination";

const userListQuerySchema = z
  .object({
    limit: z.coerce.number().int().min(MIN_LIMIT).max(MAX_LIMIT).default(DEFAULT_LIMIT),
    offset: z.coerce.number().int().min(0).max(Number.MAX_SAFE_INTEGER).optional(),
    page: z.coerce.number().int().min(1).max(MAX_PAGE).optional(),
  })
  .refine((query) => !(query.offset !== undefined && query.page !== undefined), {
    message: "Cannot combine ?offset= and ?page= parameters",
  });

type UserListQuery = z.infer<typeof userListQuerySchema>;

export const buildUserRouter = (userRepo: Repository<User>) => {
  const router = Router();

  // List registered users with validated limit/offset pagination
  router.get("/", validateQuery(userListQuerySchema), async (req, res, next) => {
    try {
      const { limit: rawLimit, offset: rawOffset, page: rawPage } = (
        req as Request & { validatedQuery: UserListQuery }
      ).validatedQuery;

      const { page: normalizedPage, limit } = normalizePagination(rawPage, rawLimit);
      const offset =
        rawOffset !== undefined
          ? rawOffset
          : getPaginationOffset(normalizedPage, limit);
      const effectivePage =
        rawOffset !== undefined
          ? Math.floor(offset / limit) + 1
          : normalizedPage;

      const [users, total] = await userRepo.findAndCount({
        order: { id: "ASC" },
        skip: offset,
        take: limit,
      });

      res.json({
        data: users,
        meta: {
          ...calculatePaginationMeta(total, effectivePage, limit),
          offset,
        },
      });
    } catch (error) {
      next(error);
    }
  });

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
