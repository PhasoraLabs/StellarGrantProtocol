/**
 * TransactionTracker — Optimistic UI State Management (#487)
 *
 * Provides event-driven transaction lifecycle tracking for responsive UIs.
 * Emits events at each stage: signed, submitted, confirmed, failed.
 *
 * Usage:
 * ```typescript
 * const tracker = new TransactionTracker();
 *
 * tracker.on('signed', (txId) => console.log('Transaction signed:', txId));
 * tracker.on('submitted', (txId, hash) => console.log('Submitted:', hash));
 * tracker.on('confirmed', (txId, result) => updateUI(result));
 * tracker.on('failed', (txId, error) => showError(error));
 *
 * const txId = tracker.track(async () => {
 *   return await sdk.grantCreate(input);
 * });
 * ```
 */

export type TransactionStage = 'pending' | 'signed' | 'submitted' | 'confirmed' | 'failed';

export interface TransactionTrackerEvents {
  signed: (txId: string) => void;
  submitted: (txId: string, hash: string) => void;
  confirmed: (txId: string, result: any) => void;
  failed: (txId: string, error: Error) => void;
  stageChange: (txId: string, stage: TransactionStage) => void;
}

export interface OptimisticUpdate<T = any> {
  txId: string;
  stage: TransactionStage;
  predictedState?: T;
  timestamp: number;
  hash?: string;
  result?: any;
  error?: Error;
}

type EventListener<T extends keyof TransactionTrackerEvents> = TransactionTrackerEvents[T];

export class TransactionTracker {
  private listeners: Map<keyof TransactionTrackerEvents, Set<Function>> = new Map();
  private transactions: Map<string, OptimisticUpdate> = new Map();
  private txCounter = 0;

  /**
   * Register an event listener for a specific transaction stage.
   */
  on<T extends keyof TransactionTrackerEvents>(
    event: T,
    listener: EventListener<T>
  ): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(listener);

    // Return unsubscribe function
    return () => {
      this.listeners.get(event)?.delete(listener);
    };
  }

  /**
   * Remove an event listener.
   */
  off<T extends keyof TransactionTrackerEvents>(
    event: T,
    listener: EventListener<T>
  ): void {
    this.listeners.get(event)?.delete(listener);
  }

  /**
   * Emit an event to all registered listeners.
   */
  private emit<T extends keyof TransactionTrackerEvents>(
    event: T,
    ...args: Parameters<EventListener<T>>
  ): void {
    const listeners = this.listeners.get(event);
    if (listeners) {
      listeners.forEach((listener) => {
        try {
          (listener as any)(...args);
        } catch (error) {
          console.error(`Error in ${event} listener:`, error);
        }
      });
    }
  }

  /**
   * Track a transaction through its lifecycle.
   * Returns a unique transaction ID for tracking.
   */
  async track<T = any>(
    txFn: () => Promise<any>,
    predictedState?: T
  ): Promise<string> {
    const txId = `tx_${++this.txCounter}_${Date.now()}`;

    this.updateStage(txId, 'pending', { predictedState });

    try {
      // Stage 1: Signed
      this.updateStage(txId, 'signed');
      this.emit('signed', txId);

      // Execute the transaction
      const result = await txFn();

      // Stage 2: Submitted
      const hash = result?.hash || result?.id || 'unknown';
      this.updateStage(txId, 'submitted', { hash });
      this.emit('submitted', txId, hash);

      // Stage 3: Confirmed (if result indicates success)
      if (result?.status === 'SUCCESS' || result?.ledger) {
        this.updateStage(txId, 'confirmed', { result });
        this.emit('confirmed', txId, result);
      }

      return txId;
    } catch (error) {
      // Stage 4: Failed
      const err = error instanceof Error ? error : new Error(String(error));
      this.updateStage(txId, 'failed', { error: err });
      this.emit('failed', txId, err);
      throw error;
    }
  }

  /**
   * Update the stage of a transaction.
   */
  private updateStage(
    txId: string,
    stage: TransactionStage,
    updates: Partial<OptimisticUpdate> = {}
  ): void {
    const existing = this.transactions.get(txId);
    const updated: OptimisticUpdate = {
      txId,
      stage,
      timestamp: Date.now(),
      ...existing,
      ...updates,
    };

    this.transactions.set(txId, updated);
    this.emit('stageChange', txId, stage);
  }

  /**
   * Get the current state of a transaction.
   */
  getTransaction(txId: string): OptimisticUpdate | undefined {
    return this.transactions.get(txId);
  }

  /**
   * Get all tracked transactions.
   */
  getAllTransactions(): OptimisticUpdate[] {
    return Array.from(this.transactions.values());
  }

  /**
   * Get transactions by stage.
   */
  getTransactionsByStage(stage: TransactionStage): OptimisticUpdate[] {
    return Array.from(this.transactions.values()).filter(tx => tx.stage === stage);
  }

  /**
   * Clear a transaction from tracking.
   */
  clear(txId: string): void {
    this.transactions.delete(txId);
  }

  /**
   * Clear all transactions.
   */
  clearAll(): void {
    this.transactions.clear();
  }

  /**
   * Remove all event listeners.
   */
  removeAllListeners(): void {
    this.listeners.clear();
  }
}
