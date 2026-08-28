/**
 * StellarGrants SDK — Error Classes
 *
 * Provides a typed error hierarchy for the StellarGrants SDK:
 *
 *   StellarGrantsError          — base class for all SDK errors
 *   ├── SorobanContractError    — mapped from a Soroban numeric error code
 *   └── StellarGrantsNetworkError — wraps network/transport failures
 *
 * All classes carry the original `cause` so that stack traces are preserved
 * for debugging while still surfacing a human-readable `message`.
 *
 * @module stellargrant-fe/lib/errors/StellarGrantsError
 */

import { ErrorCode, ERROR_MESSAGES } from "./errorCodes";
import type { ErrorCodeValue } from "./errorCodes";

// ── Base error ────────────────────────────────────────────────────────────────

/**
 * Base class for all errors thrown by the StellarGrants SDK.
 *
 * Extends the native `Error` so it passes `instanceof Error` checks and
 * works correctly with every logging / monitoring tool.
 */
export class StellarGrantsError extends Error {
  /** Discriminator tag for easy type-narrowing */
  readonly _tag = "StellarGrantsError" as const;

  /** Classification for UI messaging */
  readonly type: "network" | "api" | "rpc" | "generic";

  constructor(
    message: string,
    options?: ErrorOptions & { type?: "network" | "api" | "rpc" | "generic" }
  ) {
    super(message, options);
    this.name = "StellarGrantsError";
    this.type = options?.type ?? "generic";
    // Maintain a proper prototype chain in transpiled ES5 environments
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// ── Contract error ────────────────────────────────────────────────────────────

/**
 * Thrown when a Soroban contract invocation returns a known error code.
 *
 * Includes:
 *   - `code`      — the numeric error code from `ErrorCode`
 *   - `message`   — the human-readable description from `ERROR_MESSAGES`
 *   - `sorobanDetails` — the raw Soroban error payload for debugging
 *
 * @example
 * ```ts
 * try {
 *   await contractClient.grantFund(...);
 * } catch (err) {
 *   if (err instanceof SorobanContractError) {
 *     console.error(err.code, err.message);
 *     // e.g. 20  "Insufficient funds to complete this transaction."
 *   }
 * }
 * ```
 */
export class SorobanContractError extends StellarGrantsError {
  /** The numeric contract error code */
  readonly code: ErrorCodeValue;

  /**
   * Raw Soroban error payload as received from the RPC or SDK.
   * Useful for debugging and telemetry; do not surface to end-users.
   */
  readonly sorobanDetails: unknown;

  constructor(code: ErrorCodeValue, sorobanDetails?: unknown) {
    const message =
      ERROR_MESSAGES[code] ??
      `Unknown contract error (code ${code}).`;

    super(message, { cause: sorobanDetails, type: "rpc" });
    this.name = "SorobanContractError";
    this.code = code;
    this.sorobanDetails = sorobanDetails;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// ── Network error ─────────────────────────────────────────────────────────────

/**
 * Thrown when a network or transport failure occurs (e.g. Horizon 5xx,
 * connection refused, or a fetch timeout) that is not a contract error.
 */
export class StellarGrantsNetworkError extends StellarGrantsError {
  /** HTTP status code, if available */
  readonly statusCode?: number;

  constructor(
    message: string,
    options?: ErrorOptions & {
      statusCode?: number;
      type?: "network" | "api" | "rpc" | "generic";
    }
  ) {
    super(message, {
      ...options,
      type: options?.type ?? (options?.statusCode ? "api" : "network"),
    });
    this.name = "StellarGrantsNetworkError";
    this.statusCode = options?.statusCode;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// ── Well-known typed constructors ─────────────────────────────────────────────
// Convenience factories so call-sites don't need to import ErrorCode directly.

/** Grant not found */
export const Errors = {
  grantNotFound: (details?: unknown) =>
    new SorobanContractError(ErrorCode.GrantNotFound, details),

  unauthorized: (details?: unknown) =>
    new SorobanContractError(ErrorCode.Unauthorized, details),

  insufficientFunds: (details?: unknown) =>
    new SorobanContractError(ErrorCode.InsufficientFunds, details),

  contractPaused: (details?: unknown) =>
    new SorobanContractError(ErrorCode.ContractPaused, details),

  milestoneNotFound: (details?: unknown) =>
    new SorobanContractError(ErrorCode.MilestoneNotFound, details),

  quorumNotReached: (details?: unknown) =>
    new SorobanContractError(ErrorCode.QuorumNotReached, details),

  alreadyVoted: (details?: unknown) =>
    new SorobanContractError(ErrorCode.AlreadyVoted, details),

  hardCapExceeded: (details?: unknown) =>
    new SorobanContractError(ErrorCode.HardCapExceeded, details),

  network: (message: string, statusCode?: number, cause?: unknown) =>
    new StellarGrantsNetworkError(message, { statusCode, cause }),
} as const;
