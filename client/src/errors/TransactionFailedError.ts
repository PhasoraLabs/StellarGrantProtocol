import { StellarGrantsError } from "./StellarGrantsError";

export class TransactionFailedError extends StellarGrantsError {
  readonly hash: string;
  readonly errorResult?: unknown;

  constructor(hash: string, errorResult?: unknown, details?: unknown) {
    super(
      `Transaction ${hash} failed`,
      "TRANSACTION_FAILED",
      details,
    );
    this.name = "TransactionFailedError";
    this.hash = hash;
    this.errorResult = errorResult;
  }
}
