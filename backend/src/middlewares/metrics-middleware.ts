import { NextFunction, Request, Response } from "express";
import { metricsService } from "../services/metrics-service";

const routeLabel = (req: Request): string => {
  if (req.route?.path) {
    return req.baseUrl ? `${req.baseUrl}${req.route.path}` : String(req.route.path);
  }
  return req.path || "unknown";
};

export const metricsMiddleware = (req: Request, res: Response, next: NextFunction) => {
  const start = process.hrtime.bigint();

  res.on("finish", () => {
    const end = process.hrtime.bigint();
    const durationMs = Number(end - start) / 1_000_000;
    metricsService.observeHttpRequest(req.method, routeLabel(req), res.statusCode, durationMs);
  });

  next();
};
