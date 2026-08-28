import { ContractErrorCode, ErrorMessages } from "./errorCodes";

/**
 * Base class for all SDK-level errors.
 *
 * Every error thrown by StellarGrantsSDK extends this class so callers can
 * use a single `instanceof StellarGrantsError` guard to distinguish SDK
 * errors from unexpected runtime exceptions.
 */
export class StellarGrantsError extends Error {
  /** A stable, machine-readable identifier for the error category. */
  readonly code: string;

  /**
   * Optional structured payload attached by the thrower.
   * Use this for debugging — it may contain raw RPC responses, original
   * Error objects, validation failure lists, etc.
   */
  readonly details?: unknown;

  constructor(message: string, code = "STELLAR_GRANTS_ERROR", details?: unknown) {
    super(message);
    this.name = "StellarGrantsError";
    this.code = code;
    this.details = details;
  }
}

/**
 * Thrown when a Soroban host function invocation fails with a generic
 * revert or `txFailed` result that cannot be attributed to a specific
 * contract error code.
 */
export class SorobanRevertError extends StellarGrantsError {
  constructor(message: string, details?: unknown) {
    super(message, "SOROBAN_REVERT", details);
    this.name = "SorobanRevertError";
  }
}

/**
 * Thrown when the Soroban host reports a typed contract error — i.e. the
 * contract panicked with a value from its `#[contracterror]` enum.
 *
 * The `contractCode` property holds the strongly-typed `ContractErrorCode`
 * variant so callers can branch on specific failure cases without string
 * matching.
 *
 * @example
 * ```ts
 * import { ContractError, ContractErrorCode } from "@stellargrants/client";
 *
 * try {
 *   await sdk.voteOnMilestone(params);
 * } catch (err) {
 *   if (err instanceof ContractError) {
 *     if (err.contractCode === ContractErrorCode.AlreadyVoted) {
 *       showToast("You have already voted on this milestone.");
 *     }
 *   }
 * }
 * ```
 */
export class ContractError extends StellarGrantsError {
  /**
   * The strongly-typed contract error code extracted from the Soroban error
   * response.  Maps directly to the `ContractError` enum in `types.rs`.
   */
  readonly contractCode: ContractErrorCode;

  constructor(contractCode: ContractErrorCode, sorobanDetails?: unknown) {
    const message = ErrorMessages[contractCode];
    super(message, `CONTRACT_ERROR_${contractCode}`, sorobanDetails);
    this.name = "ContractError";
    this.contractCode = contractCode;
  }
}
