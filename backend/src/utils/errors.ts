export enum ErrorCode {
  INTERNAL_ERROR = "INTERNAL_ERROR",
  NOT_FOUND = "NOT_FOUND",
  BAD_REQUEST = "BAD_REQUEST",
  UNAUTHORIZED = "UNAUTHORIZED",
  FORBIDDEN = "FORBIDDEN",
  VALIDATION_ERROR = "VALIDATION_ERROR",
  CONFLICT = "CONFLICT",
  RATE_LIMIT_EXCEEDED = "RATE_LIMIT_EXCEEDED",
}

export class AppError extends Error {
  public readonly statusCode: number;
  public readonly errorCode: ErrorCode;
  public readonly isOperational: boolean;
  public readonly details?: unknown;

  constructor(
    message: string,
    statusCode: number,
    errorCode: ErrorCode,
    isOperational = true,
    details?: unknown
  ) {
    super(message);
    this.statusCode = statusCode;
    this.errorCode = errorCode;
    this.isOperational = isOperational;
    this.details = details;

    Error.captureStackTrace(this, this.constructor);
    Object.setPrototypeOf(this, AppError.prototype);
  }

  toJSON() {
    const result: Record<string, unknown> = {
      error: true,
      code: this.errorCode,
      message: this.message,
    };
    if (this.details) {
      result.details = this.details;
    }
    return result;
  }
}

export class NotFoundError extends AppError {
  constructor(message: string = "Resource not found", details?: unknown) {
    super(message, 404, ErrorCode.NOT_FOUND, true, details);
  }
}

export class BadRequestError extends AppError {
  constructor(message: string = "Bad request", details?: unknown) {
    super(message, 400, ErrorCode.BAD_REQUEST, true, details);
  }
}

export class UnauthorizedError extends AppError {
  constructor(message: string = "Unauthorized", details?: unknown) {
    super(message, 401, ErrorCode.UNAUTHORIZED, true, details);
  }
}

export class ForbiddenError extends AppError {
  constructor(message: string = "Forbidden", details?: unknown) {
    super(message, 403, ErrorCode.FORBIDDEN, true, details);
  }
}

export class ValidationError extends AppError {
  constructor(message: string = "Validation error", details?: unknown) {
    super(message, 400, ErrorCode.VALIDATION_ERROR, true, details);
  }
}

export class ConflictError extends AppError {
  constructor(message: string = "Conflict", details?: unknown) {
    super(message, 409, ErrorCode.CONFLICT, true, details);
  }
}

export class RateLimitError extends AppError {
  constructor(message: string = "Rate limit exceeded", details?: unknown) {
    super(message, 429, ErrorCode.RATE_LIMIT_EXCEEDED, true, details);
  }
}

export class InternalError extends AppError {
  constructor(message: string = "Internal server error", details?: unknown) {
    super(message, 500, ErrorCode.INTERNAL_ERROR, false, details);
  }
}
