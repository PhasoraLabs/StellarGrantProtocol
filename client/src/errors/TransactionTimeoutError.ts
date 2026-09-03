import { StellarGrantsError } from "./StellarGrantsError";

export class TransactionTimeoutError extends StellarGrantsError {
  readonly hash: string;
  readonly timeoutMs: number;

  constructor(hash: string, timeoutMs: number, details?: unknown) {
    super(
      `Transaction ${hash} timed out after ${timeoutMs}ms`,
      "TRANSACTION_TIMEOUT",
      details,
    );
    this.name = "TransactionTimeoutError";
    this.hash = hash;
    this.timeoutMs = timeoutMs;
  }
}
