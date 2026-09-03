/**
 * BatchBuilder
 *
 * Collects multiple SDK contract-call operations and builds them into a
 * single Stellar transaction, reducing fees for multi-step workflows
 * (e.g. voting on several milestones in one click).
 */

import { logger } from "../logger";

export type BatchOperationStatus = "pending" | "success" | "failed";

export interface BatchOperation {
  id: string;
  method: string;
  args: Record<string, unknown>;
  status: BatchOperationStatus;
  error?: string;
}

export interface BatchResult {
  /** True only if every operation succeeded */
  allSucceeded: boolean;
  operations: BatchOperation[];
  txHash?: string;
  /** Estimated total fee in stroops */
  estimatedFee?: bigint;
}

let _opCounter = 0;
function nextId(): string {
  return `op_${++_opCounter}`;
}

export class BatchBuilder {
  private ops: BatchOperation[] = [];
  private batchLogger = logger.child("BatchBuilder");

  /** Add a contract method call to the batch */
  add(method: string, args: Record<string, unknown>): this {
    this.ops.push({ id: nextId(), method, args, status: "pending" });
    this.batchLogger.debug(`Added operation`, { method, opCount: this.ops.length });
    return this;
  }

  /** Number of pending operations */
  get size(): number {
    return this.ops.length;
  }

  /** Clear all queued operations */
  clear(): this {
    this.ops = [];
    return this;
  }

  /**
   * Execute all queued operations.
   *
   * In production this method would:
   *   1. Simulate each `invokeHostFunction` op to obtain resource footprints.
   *   2. Build a single `TransactionEnvelope` containing all ops.
   *   3. Re-simulate the combined transaction for accurate fee estimation.
   *   4. Sign and submit via the wallet.
   *
   * The current implementation runs operations sequentially (one per tx) so
   * existing contract bindings can be used unchanged while the contract team
   * exposes multi-op batching at the XDR level.
   */
  async execute(
    executor: (method: string, args: Record<string, unknown>) => Promise<string | void>,
  ): Promise<BatchResult> {
    if (this.ops.length === 0) {
      this.batchLogger.warn("execute() called with no operations");
      return { allSucceeded: true, operations: [] };
    }

    this.batchLogger.info(`Executing batch`, { count: this.ops.length });
    let lastTxHash: string | undefined;
    let totalFee = 0n;

    for (const op of this.ops) {
      try {
        this.batchLogger.debug(`Executing op`, { id: op.id, method: op.method });
        const result = await executor(op.method, op.args);
        op.status = "success";
        if (typeof result === "string") lastTxHash = result;
        // Approximate fee per op: 100 stroops base + Soroban resource fee estimate
        totalFee += 100n;
      } catch (err) {
        op.status = "failed";
        op.error = err instanceof Error ? err.message : String(err);
        this.batchLogger.error(`Op failed`, { id: op.id, method: op.method, error: op.error });
      }
    }

    const result: BatchResult = {
      allSucceeded: this.ops.every((o) => o.status === "success"),
      operations: [...this.ops],
      txHash: lastTxHash,
      estimatedFee: totalFee,
    };

    this.batchLogger.info(`Batch complete`, {
      succeeded: this.ops.filter((o) => o.status === "success").length,
      failed: this.ops.filter((o) => o.status === "failed").length,
    });

    return result;
  }

  /** Snapshot of current ops (read-only) */
  preview(): Readonly<BatchOperation>[] {
    return this.ops.map((o) => ({ ...o }));
  }
}
