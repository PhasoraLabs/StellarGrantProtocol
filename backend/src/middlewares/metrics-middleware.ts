import { NextFunction, Request, Response } from "express";
import { metricsService } from "../services/metrics-service";

/**
 * Returns the route template (e.g. "/grants/:id") rather than the raw URL
 * path so that Prometheus cardinality stays bounded.  Must be called after
 * the request has been dispatched to a router (i.e. inside the "finish"
 * listener, not synchronously when the middleware first runs) so that
 * req.route is already populated.
 */
const routeLabel = (req: Request): string => {
  if (req.route?.path) {
    return req.baseUrl ? `${req.baseUrl}${req.route.path}` : String(req.route.path);
  }
  return req.path || "unknown";
};

export const metricsMiddleware = (req: Request, res: Response, next: NextFunction) => {
  const start = process.hrtime.bigint();
  const method = req.method;

  res.on("finish", () => {
    const end = process.hrtime.bigint();
    const durationMs = Number(end - start) / 1_000_000;
    // routeLabel is evaluated here — after routing — so req.route is set and
    // the label reflects the template (e.g. "/grants/:id") not the raw path.
    const route = routeLabel(req);
    const statusCode = res.statusCode;
    metricsService.observeHttpRequest(method, route, statusCode, durationMs);
  });

  next();
};
