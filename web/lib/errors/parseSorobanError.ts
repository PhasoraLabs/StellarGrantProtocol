/**
 * StellarGrants SDK — Soroban Error Parser
 *
 * Translates raw Soroban / Horizon error payloads into typed
 * `StellarGrantsError` subclasses. This is the single entry point that
 * all SDK methods should call when they catch an exception from the
 * Stellar SDK.
 *
 * Supports several Soroban error shapes:
 *
 *   1. `{ code: number }`              — numeric Soroban error code
 *   2. `{ type: "contract", value: N }`— stellar-sdk ≥ 11 shape
 *   3. `{ message: "Error(Contract, #N)" }` — stringified Soroban error
 *   4. Horizon `{ status: 4xx/5xx }`   — network-level failure
 *   5. Anything else                    — generic StellarGrantsError
 *
 * @module web/lib/errors/parseSorobanError
 */

import { ErrorCode, ERROR_MESSAGES } from "./errorCodes";
import type { ErrorCodeValue } from "./errorCodes";
import {
  StellarGrantsError,
  SorobanContractError,
  StellarGrantsNetworkError,
} from "./StellarGrantsError";

// ── Shape detectors ───────────────────────────────────────────────────────────

/** Matches Soroban SDK error payloads that contain a numeric `code` field. */
function hasNumericCode(
  value: unknown
): value is { code: number } {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof (value as Record<string, unknown>).code === "number"
  );
}

/**
 * Matches stellar-sdk ≥ 11 structured contract errors:
 * `{ type: "contract", value: <number> }`
 */
function isStellarSdkContractError(
  value: unknown
): value is { type: "contract"; value: number } {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as Record<string, unknown>).type === "contract" &&
    typeof (value as Record<string, unknown>).value === "number"
  );
}

/** Check if `value` has an HTTP status code (Horizon errors). */
function hasStatusCode(
  value: unknown
): value is { status: number; message?: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    "status" in value &&
    typeof (value as Record<string, unknown>).status === "number"
  );
}

/**
 * Attempt to parse an error code from an error message string.
 * Soroban returns messages like: `"Error(Contract, #10)"` or `"contract error 10"`.
 */
function parseCodeFromMessage(msg: string): number | null {
  // "Error(Contract, #N)"
  const angleMatch = /#(\d+)/.exec(msg);
  if (angleMatch) return parseInt(angleMatch[1], 10);

  // "contract error N" or "error code N"
  const plainMatch = /(?:contract\s+error|error\s+code)\s+(\d+)/i.exec(msg);
  if (plainMatch) return parseInt(plainMatch[1], 10);

  return null;
}

/** True if the numeric code is a valid registered ErrorCode. */
function isKnownCode(code: number): code is ErrorCodeValue {
  return Object.values(ErrorCode).includes(code as ErrorCodeValue);
}

// ── Main parser ───────────────────────────────────────────────────────────────

/**
 * Parse any thrown value into a typed `StellarGrantsError`.
 *
 * Call this at every SDK boundary to convert raw network / contract errors
 * into the SDK's typed error hierarchy.
 *
 * @example
 * ```ts
 * import { parseSorobanError } from "@/lib/errors";
 *
 * try {
 *   await contractClient.grantFund(...);
 * } catch (err) {
 *   throw parseSorobanError(err);
 * }
 * ```
 *
 * @param raw - Any value thrown or rejected by the Stellar SDK or fetch
 * @returns A typed StellarGrantsError subclass
 */
export function parseSorobanError(raw: unknown): StellarGrantsError {
  // ── Already typed — pass through ────────────────────────────────────────
  if (raw instanceof StellarGrantsError) return raw;

  // ── stellar-sdk ≥ 11: { type: "contract", value: N } ───────────────────
  if (isStellarSdkContractError(raw)) {
    const code = raw.value;
    if (isKnownCode(code)) return new SorobanContractError(code, raw);
    return new StellarGrantsError(
      `Unknown contract error code ${code}.`,
      { cause: raw }
    );
  }

  // ── Numeric code field: { code: N, ... } ────────────────────────────────
  if (hasNumericCode(raw)) {
    const code = raw.code;
    if (isKnownCode(code)) return new SorobanContractError(code, raw);
    return new StellarGrantsError(
      `Unknown contract error code ${code}.`,
      { cause: raw }
    );
  }

  // ── Horizon HTTP error: { status: 4xx/5xx, message? } ───────────────────
  if (hasStatusCode(raw)) {
    const status = raw.status;
    const msg =
      typeof raw.message === "string" && raw.message.length > 0
        ? raw.message
        : `Network request failed with status ${status}.`;
    return new StellarGrantsNetworkError(msg, { statusCode: status, cause: raw });
  }

  // ── Error/string with parseable code in message ─────────────────────────
  const message =
    raw instanceof Error
      ? raw.message
      : typeof raw === "string"
      ? raw
      : null;

  if (message !== null) {
    const code = parseCodeFromMessage(message);
    if (code !== null && isKnownCode(code)) {
      return new SorobanContractError(code, raw);
    }

    // Unrecognised message — wrap as generic SDK error
    return new StellarGrantsError(message, { cause: raw });
  }

  // ── Completely unknown ───────────────────────────────────────────────────
  return new StellarGrantsError(
    "An unexpected error occurred in the StellarGrants SDK.",
    { cause: raw }
  );
}

/**
 * Narrow a `StellarGrantsError` to a `SorobanContractError` with a
 * specific error code. Useful in catch blocks when you only care about
 * one particular failure mode.
 *
 * @example
 * ```ts
 * const mapped = parseSorobanError(err);
 * if (isContractError(mapped, ErrorCode.InsufficientFunds)) {
 *   showToast("Not enough XLM in your wallet.");
 * }
 * ```
 */
export function isContractError(
  err: StellarGrantsError,
  code: ErrorCodeValue
): err is SorobanContractError {
  return err instanceof SorobanContractError && err.code === code;
}

/**
 * Returns the human-readable message for a known error code, or a
 * generic fallback.
 */
export function getErrorMessage(code: number): string {
  if (isKnownCode(code)) return ERROR_MESSAGES[code];
  return `Unknown contract error (code ${code}).`;
}
