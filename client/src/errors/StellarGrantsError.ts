export class StellarGrantsError extends Error {
  readonly code: string;
  readonly details?: unknown;

  constructor(message: string, code = "STELLAR_GRANTS_ERROR", details?: unknown) {
    super(message);
    this.name = "StellarGrantsError";
    this.code = code;
    this.details = details;
  }
}

export class SorobanRevertError extends StellarGrantsError {
  constructor(message: string, details?: unknown) {
    super(message, "SOROBAN_REVERT", details);
    this.name = "SorobanRevertError";
  }
}
