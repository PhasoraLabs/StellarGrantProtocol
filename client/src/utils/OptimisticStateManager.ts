/**
 * OptimisticStateManager — Predict and rollback UI state (#487)
 *
 * Manages optimistic updates for grant operations with automatic rollback
 * on transaction failure.
 *
 * Usage:
 * ```typescript
 * const manager = new OptimisticStateManager();
 *
 * // Predict state after grant creation
 * const predicted = manager.predictGrantCreate({
 *   owner: 'G...',
 *   title: 'New Grant',
 *   budget: BigInt(1000000),
 *   // ...
 * });
 *
 * // Apply optimistic update
 * manager.apply('tx_123', predicted);
 *
 * // On success: commit
 * manager.commit('tx_123', actualResult);
 *
 * // On failure: rollback
 * manager.rollback('tx_123');
 * ```
 */

import type { GrantCreateInput, GrantFundInput } from "../types";

export interface OptimisticGrant {
  id: number;
  owner: string;
  title: string;
  description: string;
  budget: bigint;
  raised: bigint;
  deadline: bigint;
  milestoneCount: number;
  status: 'pending' | 'active' | 'completed' | 'cancelled';
  createdAt: number;
  optimistic?: boolean;
}

export interface OptimisticMilestone {
  grantId: number;
  index: number;
  title: string;
  description: string;
  amount: bigint;
  status: 'pending' | 'submitted' | 'approved' | 'rejected';
  submittedAt?: number;
  approvedAt?: number;
  optimistic?: boolean;
}

interface OptimisticOperation {
  txId: string;
  type: 'grant_create' | 'grant_fund' | 'milestone_submit' | 'milestone_vote';
  predictedState: any;
  previousState?: any;
  timestamp: number;
}

export class OptimisticStateManager {
  private operations: Map<string, OptimisticOperation> = new Map();
  private stateSnapshots: Map<string, any> = new Map();

  /**
   * Predict the state after a grant creation.
   */
  predictGrantCreate(input: GrantCreateInput): OptimisticGrant {
    return {
      id: -1, // Temporary ID until confirmed
      owner: input.owner,
      title: input.title,
      description: input.description,
      budget: input.budget,
      raised: BigInt(0),
      deadline: input.deadline,
      milestoneCount: input.milestoneCount,
      status: 'pending',
      createdAt: Date.now(),
      optimistic: true,
    };
  }

  /**
   * Predict the state after funding a grant.
   */
  predictGrantFund(
    currentGrant: OptimisticGrant,
    input: GrantFundInput
  ): OptimisticGrant {
    return {
      ...currentGrant,
      raised: currentGrant.raised + input.amount,
      status: currentGrant.raised + input.amount >= currentGrant.budget ? 'active' : currentGrant.status,
      optimistic: true,
    };
  }

  /**
   * Predict the state after milestone submission.
   */
  predictMilestoneSubmit(
    currentMilestone: OptimisticMilestone,
    proofHash: string
  ): OptimisticMilestone {
    return {
      ...currentMilestone,
      status: 'submitted',
      submittedAt: Date.now(),
      optimistic: true,
    };
  }

  /**
   * Predict the state after milestone vote.
   */
  predictMilestoneVote(
    currentMilestone: OptimisticMilestone,
    approve: boolean,
    currentVotes: { approve: number; reject: number },
    threshold: number
  ): OptimisticMilestone {
    const newVotes = {
      approve: currentVotes.approve + (approve ? 1 : 0),
      reject: currentVotes.reject + (approve ? 0 : 1),
    };

    let newStatus = currentMilestone.status;
    if (newVotes.approve >= threshold) {
      newStatus = 'approved';
    } else if (newVotes.reject >= threshold) {
      newStatus = 'rejected';
    }

    return {
      ...currentMilestone,
      status: newStatus,
      approvedAt: newStatus === 'approved' ? Date.now() : currentMilestone.approvedAt,
      optimistic: true,
    };
  }

  /**
   * Apply an optimistic update.
   */
  apply(
    txId: string,
    predictedState: any,
    type: OptimisticOperation['type'],
    previousState?: any
  ): void {
    this.operations.set(txId, {
      txId,
      type,
      predictedState,
      previousState,
      timestamp: Date.now(),
    });

    if (previousState) {
      this.stateSnapshots.set(txId, previousState);
    }
  }

  /**
   * Commit an optimistic update (transaction succeeded).
   * Replaces predicted state with actual result.
   */
  commit(txId: string, actualState: any): void {
    const operation = this.operations.get(txId);
    if (operation) {
      operation.predictedState = { ...actualState, optimistic: false };
    }
    this.stateSnapshots.delete(txId);
  }

  /**
   * Rollback an optimistic update (transaction failed).
   * Returns the previous state if available.
   */
  rollback(txId: string): any | undefined {
    const operation = this.operations.get(txId);
    const previousState = this.stateSnapshots.get(txId);

    this.operations.delete(txId);
    this.stateSnapshots.delete(txId);

    return previousState;
  }

  /**
   * Get the predicted state for a transaction.
   */
  getPredictedState(txId: string): any | undefined {
    return this.operations.get(txId)?.predictedState;
  }

  /**
   * Get all pending optimistic operations.
   */
  getPendingOperations(): OptimisticOperation[] {
    return Array.from(this.operations.values());
  }

  /**
   * Check if a transaction has a pending optimistic update.
   */
  hasPendingUpdate(txId: string): boolean {
    return this.operations.has(txId);
  }

  /**
   * Clear all optimistic updates.
   */
  clearAll(): void {
    this.operations.clear();
    this.stateSnapshots.clear();
  }

  /**
   * Clear old operations (older than specified milliseconds).
   */
  clearOld(maxAgeMs: number = 300000): void {
    const now = Date.now();
    for (const [txId, operation] of this.operations.entries()) {
      if (now - operation.timestamp > maxAgeMs) {
        this.operations.delete(txId);
        this.stateSnapshots.delete(txId);
      }
    }
  }
}
