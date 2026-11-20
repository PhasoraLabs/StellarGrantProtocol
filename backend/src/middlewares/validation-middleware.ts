import { Request, Response, NextFunction } from "express";
import { ZodError, ZodSchema } from "zod";

/**
 * Validation error response format
 */
interface ValidationError {
  error: string;
  details: Array<{
    field: string;
    message: string;
    code?: string;
  }>;
}

/**
 * Formats Zod validation errors into a structured response
 */
function formatValidationError(error: ZodError): ValidationError {
  const details = error.issues.map((issue) => ({
    field: issue.path.join("."),
    message: issue.message,
    code: issue.code,
  }));

  return {
    error: "Validation failed",
    details,
  };
}

/**
 * Creates a middleware that validates the request body against a Zod schema
 */
export function validateBody<T>(schema: ZodSchema<T>) {
  return (req: Request, res: Response, next: NextFunction): void => {
    const result = schema.safeParse(req.body);

    if (!result.success) {
      const formattedError = formatValidationError(result.error);
      res.status(400).json(formattedError);
      return;
    }

    // Attach validated data to request for type safety
    (req as any).validatedBody = result.data;
    next();
  };
}

/**
 * Creates a middleware that validates request query parameters against a Zod schema
 */
export function validateQuery<T>(schema: ZodSchema<T>) {
  return (req: Request, res: Response, next: NextFunction): void => {
    const result = schema.safeParse(req.query);

    if (!result.success) {
      const formattedError = formatValidationError(result.error);
      res.status(400).json(formattedError);
      return;
    }

    // Attach validated data to request for type safety
    (req as any).validatedQuery = result.data;
    next();
  };
}

/**
 * Creates a middleware that validates request parameters against a Zod schema
 */
export function validateParams<T>(schema: ZodSchema<T>) {
  return (req: Request, res: Response, next: NextFunction): void => {
    const result = schema.safeParse(req.params);

    if (!result.success) {
      const formattedError = formatValidationError(result.error);
      res.status(400).json(formattedError);
      return;
    }

    // Attach validated data to request for type safety
    (req as any).validatedParams = result.data;
    next();
  };
}

/**
 * Creates a middleware that validates body, query, and params against their respective schemas
 */
export function validateRequest<TBody, TQuery, TParams>(
  schemas: {
    body?: ZodSchema<TBody>;
    query?: ZodSchema<TQuery>;
    params?: ZodSchema<TParams>;
  }
) {
  return (req: Request, res: Response, next: NextFunction): void => {
    if (schemas.body) {
      const bodyResult = schemas.body.safeParse(req.body);
      if (!bodyResult.success) {
        const formattedError = formatValidationError(bodyResult.error);
        res.status(400).json(formattedError);
        return;
      }
      (req as any).validatedBody = bodyResult.data;
    }

    if (schemas.query) {
      const queryResult = schemas.query.safeParse(req.query);
      if (!queryResult.success) {
        const formattedError = formatValidationError(queryResult.error);
        res.status(400).json(formattedError);
        return;
      }
      (req as any).validatedQuery = queryResult.data;
    }

    if (schemas.params) {
      const paramsResult = schemas.params.safeParse(req.params);
      if (!paramsResult.success) {
        const formattedError = formatValidationError(paramsResult.error);
        res.status(400).json(formattedError);
        return;
      }
      (req as any).validatedParams = paramsResult.data;
    }

    next();
  };
}
