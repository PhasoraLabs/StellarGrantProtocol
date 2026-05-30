import { SorobanRevertError, StellarGrantsError } from "./StellarGrantsError";

/**
 * Converts generic Soroban failures into readable typed errors.
 */
export function parseSorobanError(error: unknown): Error {
  if (error instanceof Error) {
    const msg = error.message;
    const lower = msg.toLowerCase();
    if (lower.includes("revert") || lower.includes("txfailed")) {
      return new SorobanRevertError(humanizeRevertMessage(msg), { raw: msg });
    }
    return new StellarGrantsError(msg, "RPC_ERROR");
  }

  return new StellarGrantsError("Unknown Soroban RPC failure", "UNKNOWN_RPC_ERROR", error);
}

function humanizeRevertMessage(raw: string): string {
  const reasonMatch = raw.match(/revert(?:ed)?[:\s-]+(.+)/i);
  if (reasonMatch?.[1]) {
    return `Contract reverted: ${reasonMatch[1]}`;
  }

  const txFailedMatch = raw.match(/txfailed[:\s-]+(.+)/i);
  if (txFailedMatch?.[1]) {
    return `Transaction failed: ${txFailedMatch[1]}`;
  }

  return "Contract reverted while executing request";
}
