import { Request, Response, NextFunction } from "express";
import { logger } from "../config/logger";
import { env } from "../config/env";
import { AppError } from "../utils/errors";

export const errorHandler = (
  err: unknown,
  req: Request,
  res: Response,
  _next: NextFunction
): void => {
  const requestId = req.headers["x-request-id"] || "unknown";

  if (err instanceof AppError) {
    logger.error({
      message: err.message,
      errorCode: err.errorCode,
      statusCode: err.statusCode,
      requestId,
      path: req.path,
      method: req.method,
      ...(err.isOperational ? {} : { stack: err.stack }),
    });

    res.status(err.statusCode).json(err.toJSON());
    return;
  }

  if (err instanceof Error) {
    logger.error({
      message: err.message,
      requestId,
      path: req.path,
      method: req.method,
      stack: env.nodeEnv === "development" ? err.stack : undefined,
    });

    const statusCode = (err as any).statusCode || 500;
    res.status(statusCode).json({
      error: true,
      code: "INTERNAL_ERROR",
      message: env.nodeEnv === "production" ? "Internal server error" : err.message,
    });
    return;
  }

  logger.error({
    message: "Unknown error",
    error: err,
    requestId,
    path: req.path,
    method: req.method,
  });

  res.status(500).json({
    error: true,
    code: "INTERNAL_ERROR",
    message: "Internal server error",
  });
};

export const notFoundHandler = (req: Request, res: Response): void => {
  res.status(404).json({
    error: true,
    code: "NOT_FOUND",
    message: `Route ${req.method} ${req.path} not found`,
  });
};
