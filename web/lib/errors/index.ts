/**
 * StellarGrants SDK — Error Module Barrel
 *
 * Re-exports everything needed to handle SDK errors from a single import:
 *
 * ```ts
 * import {
 *   parseSorobanError,
 *   isContractError,
 *   SorobanContractError,
 *   StellarGrantsError,
 *   ErrorCode,
 *   Errors,
 * } from "@/lib/errors";
 * ```
 *
 * @module web/lib/errors
 */

export { ErrorCode, ERROR_MESSAGES } from "./errorCodes";
export type { ErrorCodeValue } from "./errorCodes";

export {
  StellarGrantsError,
  SorobanContractError,
  StellarGrantsNetworkError,
  Errors,
} from "./StellarGrantsError";

export {
  parseSorobanError,
  isContractError,
  getErrorMessage,
} from "./parseSorobanError";
