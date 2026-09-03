/**
 * Optimistic UI State Management Utilities
 *
 * Helps frontend developers manage UI state during the lag between
 * transaction submission and finalization on the Stellar blockchain.
 *
 * Provides:
 * - TransactionTracker  — event-based tracker for tx lifecycle stages
 * - predictGrantState   — predict post-tx grant state for instant UI updates
 * - OptimisticStore     — generic store for optimistic mutations with rollback
 */

import type { Grant, Milestone, MilestoneVote } from "@/types";

// ── Transaction lifecycle types ───────────────────────────────────────────

export type TransactionStage =
  | "signed"      // wallet has signed the tx
  | "submitted"   // tx sent to the network
  | "confirmed"   // tx included in a ledger
  | "failed";     // tx failed or timed out

export interface TransactionEvent {
  /** Unique tracker ID */
  id: string;
  /** Current lifecycle stage */
  stage: TransactionStage;
  /** UTC timestamp of this stage transition */
  timestamp: Date;
  /** Optional tx hash (available from "submitted" onwards) */
  txHash?: string;
  /** Error message if stage === "failed" */
  error?: string;
  /** Arbitrary metadata attached at creation */
  meta?: Record<string, unknown>;
}

export type TransactionListener = (event: TransactionEvent) => void;

// ── TransactionTracker ────────────────────────────────────────────────────

/**
 * Tracks a single transaction through its lifecycle stages and emits
 * typed events to all registered listeners.
 *
 * Framework-agnostic — works with React, Vue, Svelte, or plain JS.
 *
 * @example
 * ```ts
 * const tracker = new TransactionTracker({ meta: { grantId: "42" } });
 * tracker.on((e) => console.log(e.stage, e.txHash));
 * tracker.advance("signed");
 * tracker.advance("submitted", { txHash: "abc123" });
 * tracker.advance("confirmed", { txHash: "abc123" });
 * ```
 */
export class TransactionTracker {
  readonly id: string;

  private listeners: Set<TransactionListener> = new Set();
  private currentStage: TransactionStage | null = null;
  private meta: Record<string, unknown>;

  constructor(options?: { id?: string; meta?: Record<string, unknown> }) {
    this.id = options?.id ?? crypto.randomUUID();
    this.meta = options?.meta ?? {};
  }

  /** Register a listener for stage transitions */
  on(listener: TransactionListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /** Remove a previously registered listener */
  off(listener: TransactionListener): void {
    this.listeners.delete(listener);
  }

  /** Advance the transaction to the next stage */
  advance(
    stage: TransactionStage,
    options?: { txHash?: string; error?: string }
  ): void {
    this.currentStage = stage;
    const event: TransactionEvent = {
      id: this.id,
      stage,
      timestamp: new Date(),
      txHash: options?.txHash,
      error: options?.error,
      meta: this.meta,
    };
    this.listeners.forEach((l) => l(event));
  }

  /** Convenience: mark as signed */
  markSigned(): void {
    this.advance("signed");
  }

  /** Convenience: mark as submitted with tx hash */
  markSubmitted(txHash: string): void {
    this.advance("submitted", { txHash });
  }

  /** Convenience: mark as confirmed */
  markConfirmed(txHash: string): void {
    this.advance("confirmed", { txHash });
  }

  /** Convenience: mark as failed with reason */
  markFailed(error: string): void {
    this.advance("failed", { error });
  }

  /** Get the current stage (null if not yet advanced) */
  get stage(): TransactionStage | null {
    return this.currentStage;
  }

  /** True if the transaction reached a terminal state */
  get isTerminal(): boolean {
    return this.currentStage === "confirmed" || this.currentStage === "failed";
  }

  /** Remove all listeners and reset */
  destroy(): void {
    this.listeners.clear();
  }
}

// ── Grant state prediction ────────────────────────────────────────────────

/** Describes a pending mutation to a grant */
export type GrantMutation =
  | { type: "fund"; amount: bigint; token?: string }
  | { type: "vote"; milestoneIdx: number; reviewer: string; approve: boolean }
  | { type: "statusChange"; newStatus: number }
  | { type: "milestoneSubmit"; milestoneIdx: number; proofHash: string }
  | { type: "milestoneApprove"; milestoneIdx: number }
  | { type: "milestoneReject"; milestoneIdx: number };

/**
 * Predict the optimistic post-transaction state of a grant.
 *
 * Returns a new Grant (and optionally updated milestones) that reflects what
 * the blockchain state *should* look like after the mutation is confirmed.
 * Use this to update the UI instantly without waiting for confirmation.
 *
 * @param current  - The current grant state
 * @param mutation - The mutation being applied
 * @param milestones - Current milestones (required for milestone mutations)
 * @returns A `{ grant, milestones }` snapshot reflecting the predicted state
 */
export function predictGrantState(
  current: Grant,
  mutation: GrantMutation,
  milestones: Milestone[] = []
): { grant: Grant; milestones: Milestone[] } {
  let grant: Grant = { ...current };
  let updatedMilestones: Milestone[] = milestones.map((m) => ({ ...m }));

  switch (mutation.type) {
    case "fund": {
      grant = {
        ...grant,
        funded: grant.funded + mutation.amount,
        // If fully funded, move to In Progress (status 2)
        status:
          grant.funded + mutation.amount >= grant.budget
            ? Math.max(grant.status, 2)
            : grant.status,
      };
      break;
    }

    case "vote": {
      // Upsert this reviewer's vote on the target milestone so the tally
      // updates instantly. Re-voting replaces the reviewer's prior entry.
      updatedMilestones = updatedMilestones.map((m) => {
        if (m.idx !== mutation.milestoneIdx) return m;
        const entry: MilestoneVote = {
          reviewer: mutation.reviewer,
          vote: mutation.approve ? "approve" : "reject",
          voted_at: BigInt(Math.floor(Date.now() / 1000)),
        };
        const votes = [...(m.votes ?? [])];
        const existing = votes.findIndex((v) => v.reviewer === mutation.reviewer);
        if (existing >= 0) votes[existing] = entry;
        else votes.push(entry);
        return { ...m, votes };
      });
      break;
    }

    case "statusChange": {
      grant = { ...grant, status: mutation.newStatus };
      break;
    }

    case "milestoneSubmit": {
      updatedMilestones = updatedMilestones.map((m) =>
        m.idx === mutation.milestoneIdx
          ? {
              ...m,
              submitted: true,
              proof_hash: mutation.proofHash,
              submitted_at: BigInt(Math.floor(Date.now() / 1000)),
            }
          : m
      );
      break;
    }

    case "milestoneApprove": {
      updatedMilestones = updatedMilestones.map((m) =>
        m.idx === mutation.milestoneIdx
          ? {
              ...m,
              approved: true,
              approved_at: BigInt(Math.floor(Date.now() / 1000)),
            }
          : m
      );
      break;
    }

    case "milestoneReject": {
      // Rollback submission so it can be re-submitted
      updatedMilestones = updatedMilestones.map((m) =>
        m.idx === mutation.milestoneIdx
          ? {
              ...m,
              submitted: false,
              proof_hash: null,
              submitted_at: null,
            }
          : m
      );
      break;
    }
  }

  return { grant, milestones: updatedMilestones };
}

// ── OptimisticStore ───────────────────────────────────────────────────────

/**
 * A generic store that holds optimistic (predicted) state alongside the
 * confirmed state, and provides rollback if a transaction fails.
 *
 * Framework-agnostic — integrates with React via the `useOptimisticGrant`
 * hook, but can be used directly in Vue, Svelte, or plain JS.
 *
 * @example
 * ```ts
 * const store = new OptimisticStore(confirmedGrant);
 * const tracker = store.applyMutation({ type: "fund", amount: 100n });
 * tracker.on((e) => {
 *   if (e.stage === "confirmed") store.commit();
 *   if (e.stage === "failed")    store.rollback();
 * });
 * ```
 */
export class OptimisticStore<T> {
  private confirmed: T;
  private optimistic: T;
  private listeners: Set<(state: T) => void> = new Set();

  constructor(initialState: T) {
    this.confirmed = { ...initialState as object } as T;
    this.optimistic = { ...initialState as object } as T;
  }

  /** Current state (optimistic if a mutation is pending, confirmed otherwise) */
  get state(): T {
    return this.optimistic;
  }

  /** The last confirmed (on-chain) state */
  get confirmedState(): T {
    return this.confirmed;
  }

  /**
   * Apply an optimistic update.
   * @param updater - Function that receives the current confirmed state and returns the predicted state
   * @returns The new optimistic state
   */
  apply(updater: (current: T) => T): T {
    this.optimistic = updater(this.confirmed);
    this.notify();
    return this.optimistic;
  }

  /**
   * Commit the optimistic state as the new confirmed state.
   * Call this when the transaction is confirmed on-chain.
   */
  commit(): void {
    this.confirmed = { ...this.optimistic as object } as T;
    this.notify();
  }

  /**
   * Roll back the optimistic state to the last confirmed state.
   * Call this when a transaction fails.
   */
  rollback(): void {
    this.optimistic = { ...this.confirmed as object } as T;
    this.notify();
  }

  /**
   * Update the confirmed state externally (e.g. after polling the chain).
   */
  setConfirmed(state: T): void {
    this.confirmed = { ...state as object } as T;
    this.optimistic = { ...state as object } as T;
    this.notify();
  }

  /** Subscribe to state changes. Returns an unsubscribe function. */
  subscribe(listener: (state: T) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify(): void {
    this.listeners.forEach((l) => l(this.optimistic));
  }
}

// ── createOptimisticGrantMutation ─────────────────────────────────────────

/**
 * High-level helper that wires a TransactionTracker to an OptimisticStore
 * for a grant mutation. Applies the optimistic state immediately and rolls
 * back automatically on failure.
 *
 * @returns The TransactionTracker so callers can add additional listeners.
 *
 * @example
 * ```ts
 * const tracker = createOptimisticGrantMutation(store, mutation, milestones);
 * // Submit tx on chain, then:
 * tracker.markSubmitted(txHash);
 * // On ledger close:
 * tracker.markConfirmed(txHash);
 * ```
 */
export function createOptimisticGrantMutation(
  store: OptimisticStore<Grant>,
  mutation: GrantMutation,
  milestones: Milestone[] = [],
  trackerOptions?: { id?: string; meta?: Record<string, unknown> }
): TransactionTracker {
  const tracker = new TransactionTracker(trackerOptions);

  // Apply optimistic state immediately
  store.apply((current) => predictGrantState(current, mutation, milestones).grant);

  tracker.on((event) => {
    if (event.stage === "confirmed") {
      store.commit();
    }
    if (event.stage === "failed") {
      store.rollback();
    }
  });

  return tracker;
}
