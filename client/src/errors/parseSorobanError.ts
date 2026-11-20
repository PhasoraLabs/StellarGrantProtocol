import { ContractError, SorobanRevertError, StellarGrantsError } from "./StellarGrantsError";
import { ContractErrorCode } from "./errorCodes";

// ---------------------------------------------------------------------------
// Pattern constants
// ---------------------------------------------------------------------------

/**
 * Matches the Soroban host error format for contract-defined errors.
 *
 * When a Soroban contract panics via `#[contracterror]`, the stellar-sdk
 * serialises the host value as:
 *
 *   `Error(Contract, #N)`
 *
 * where N is the u32 discriminant of the failing enum variant.
 *
 * This pattern also handles the common prefixes emitted by the host:
 *   - `HostError: Error(Contract, #N)`
 *   - `TransactionSubmitErrors: ... Error(Contract, #N) ...`
 */
const CONTRACT_ERROR_RE = /Error\(\s*Contract\s*,\s*#\s*(\d+)\s*\)/i;

/**
 * Matches generic Soroban revert / transaction-failure signals that do NOT
 * carry a specific contract error code.
 */
const REVERT_RE = /revert|txfailed/i;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Converts any value thrown during a Soroban invocation into a structured,
 * typed SDK error.
 *
 * Resolution order:
 * 1. If the error message contains `Error(Contract, #N)`, return a
 *    {@link ContractError} with the corresponding {@link ContractErrorCode}.
 * 2. If the error message contains `revert` or `txFailed`, return a
 *    {@link SorobanRevertError} with a humanised message.
 * 3. If a plain `Error` was thrown (e.g. network failure), return a
 *    {@link StellarGrantsError} with code `"RPC_ERROR"`.
 * 4. For any other thrown value (string, object, null, …) return a
 *    {@link StellarGrantsError} with code `"UNKNOWN_RPC_ERROR"`.
 *
 * @param error - The value caught in a `catch` block.
 * @returns A typed SDK error.  Never throws.
 *
 * @example
 * ```ts
 * try {
 *   await sdk.voteOnMilestone(params);
 * } catch (err) {
 *   const sdkError = parseSorobanError(err);
 *   console.error(sdkError.message);   // human-readable
 *   console.error(sdkError.details);   // raw Soroban payload for debugging
 * }
 * ```
 */
export function parseSorobanError(error: unknown): StellarGrantsError {
  if (error instanceof Error) {
    const msg = error.message;

    // --- Priority 1: Typed contract error --------------------------------
    const contractMatch = CONTRACT_ERROR_RE.exec(msg);
    if (contractMatch) {
      const numericCode = Number(contractMatch[1]);
      const contractCode = resolveContractCode(numericCode);

      if (contractCode !== null) {
        return new ContractError(contractCode, { raw: msg, originalError: error });
      }

      // The code is from the contract namespace but unknown to this SDK version.
      return new StellarGrantsError(
        `Contract returned an unrecognised error code: #${numericCode}`,
        `CONTRACT_ERROR_${numericCode}`,
        { raw: msg, originalError: error },
      );
    }

    // --- Priority 2: Generic revert / txFailed ---------------------------
    if (REVERT_RE.test(msg)) {
      return new SorobanRevertError(humanizeRevertMessage(msg), { raw: msg, originalError: error });
    }

    // --- Priority 3: Other plain Error (network, RPC, etc.) --------------
    return new StellarGrantsError(msg, "RPC_ERROR", { originalError: error });
  }

  // --- Priority 4: Non-Error thrown value ----------------------------------
  return new StellarGrantsError(
    "Unknown Soroban RPC failure",
    "UNKNOWN_RPC_ERROR",
    error,
  );
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Maps a raw u32 discriminant extracted from a Soroban error message to the
 * corresponding {@link ContractErrorCode} enum member.
 *
 * Returns `null` when the code is outside the set of known contract variants
 * (i.e. the SDK is out-of-date with a newer contract deployment).
 */
function resolveContractCode(numericCode: number): ContractErrorCode | null {
  // Use Object.values() to get the numeric members of the enum, then check
  // membership.  Numeric enums emit both name→value and value→name entries;
  // filtering to typeof === "number" isolates the actual discriminant values.
  const validCodes = Object.values(ContractErrorCode).filter(
    (v): v is ContractErrorCode => typeof v === "number",
  );
  if (validCodes.includes(numericCode as ContractErrorCode)) {
    return numericCode as ContractErrorCode;
  }
  return null;
}

/**
 * Produces a readable sentence from a raw revert / txFailed error string.
 */
function humanizeRevertMessage(raw: string): string {
  const reasonMatch = raw.match(/revert(?:ed)?[:\s-]+(.+)/i);
  if (reasonMatch?.[1]) {
    return `Contract reverted: ${reasonMatch[1].trim()}`;
  }

  const txFailedMatch = raw.match(/txfailed[:\s-]+(.+)/i);
  if (txFailedMatch?.[1]) {
    return `Transaction failed: ${txFailedMatch[1].trim()}`;
  }

  return "Contract reverted while executing request";
}
