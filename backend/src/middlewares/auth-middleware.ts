import { Response, NextFunction } from "express";
import { AuthenticatedRequest } from "../types/auth";
import { UnauthorizedError } from "../utils/errors";

export function authMiddleware(req: AuthenticatedRequest, _res: Response, next: NextFunction) {
  req.user = {
    stellarAddress: req.headers["x-stellar-address"] as string || "",
  };
  if (!req.user.stellarAddress) {
    return next(new UnauthorizedError());
  }
  next();
}
