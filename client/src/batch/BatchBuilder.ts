import type { xdr } from "@stellar/stellar-sdk";
import { StellarGrantsError } from "../errors/StellarGrantsError";

export type BatchCall = {
  method: string;
  args: xdr.ScVal[];
  label?: string;
};

export class BatchOperationError extends StellarGrantsError {
  public readonly operationIndex: number;
  public readonly method?: string;
  public readonly label?: string;

  constructor(message: string, opts: { operationIndex: number; method?: string; label?: string; details?: any }) {
    super(message, "BATCH_OPERATION_FAILED", opts.details);
    this.operationIndex = opts.operationIndex;
    this.method = opts.method;
    this.label = opts.label;
  }
}

export type BatchSendOptions = {
  feePriority?: "low" | "medium" | "high";
  fee?: string;
  simulatedFee?: string;
  footprint?: any;
  /** If true, return prepared unsigned XDR instead of submitting. */
  returnUnsignedXdr?: boolean;
  /** If true, wait for confirmation (only when submitting). */
  waitForConfirmation?: boolean;
  pollIntervalMs?: number;
  timeoutMs?: number;
};

/**
 * Collects multiple contract calls into a single Stellar transaction.
 *
 * The batch is simulated as a whole (to get correct resource estimates),
 * then prepared, signed, and submitted once.
 */
export class BatchBuilder {
  private readonly calls: BatchCall[] = [];

  constructor(private readonly sdk: any) {}

  add(method: string, args: xdr.ScVal[], opts?: { label?: string }): this {
    this.calls.push({ method, args, label: opts?.label });
    return this;
  }

  size(): number {
    return this.calls.length;
  }

  clear(): this {
    this.calls.length = 0;
    return this;
  }

  async simulate(options?: Omit<BatchSendOptions, "waitForConfirmation" | "returnUnsignedXdr">): Promise<any> {
    if (this.calls.length === 0) {
      throw new StellarGrantsError("Batch is empty", "BATCH_EMPTY");
    }
    return this.sdk.__simulateBatch(this.calls, options);
  }

  async send(options?: BatchSendOptions): Promise<any> {
    if (this.calls.length === 0) {
      throw new StellarGrantsError("Batch is empty", "BATCH_EMPTY");
    }
    return this.sdk.__sendBatch(this.calls, options);
  }
}

