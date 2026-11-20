"use client";

/**
 * useOptimisticGrant Hook
 *
 * React hook combining OptimisticStore + TransactionTracker for snappy UX.
 */

import { useState, useEffect, useCallback, useRef } from "react";
import type { Grant, Milestone } from "@/types";
import {
  OptimisticStore,
  TransactionTracker,
  createOptimisticGrantMutation,
  predictGrantState,
  type GrantMutation,
  type TransactionEvent,
  type TransactionStage,
} from "@/lib/utils/optimistic";

export interface UseOptimisticGrantResult {
  grant: Grant;
  milestones: Milestone[];
  isPending: boolean;
  txStage: TransactionStage | null;
  txError: string | null;
  applyMutation: (mutation: GrantMutation, options?: { meta?: Record<string, unknown> }) => TransactionTracker;
  setConfirmed: (grant: Grant, milestones?: Milestone[]) => void;
  rollback: () => void;
}

export function useOptimisticGrant(
  initialGrant: Grant,
  initialMilestones: Milestone[] = []
): UseOptimisticGrantResult {
  const grantStoreRef = useRef<OptimisticStore<Grant>>(new OptimisticStore(initialGrant));
  const [milestones, setMilestones] = useState<Milestone[]>(initialMilestones);
  const confirmedMilestonesRef = useRef<Milestone[]>(initialMilestones);
  const [grant, setGrant] = useState<Grant>(initialGrant);
  const [txStage, setTxStage] = useState<TransactionStage | null>(null);
  const [txError, setTxError] = useState<string | null>(null);

  const isPending = txStage !== null && txStage !== "confirmed" && txStage !== "failed";

  useEffect(() => {
    return grantStoreRef.current.subscribe((state) => setGrant({ ...state }));
  }, []);

  useEffect(() => {
    grantStoreRef.current.setConfirmed(initialGrant);
  }, [initialGrant]);

  const applyMutation = useCallback((
    mutation: GrantMutation,
    options?: { meta?: Record<string, unknown> }
  ): TransactionTracker => {
    const store = grantStoreRef.current;
    const { milestones: predicted } = predictGrantState(store.confirmedState, mutation, confirmedMilestonesRef.current);
    setMilestones(predicted);

    const tracker = createOptimisticGrantMutation(store, mutation, [], options);

    tracker.on((event: TransactionEvent) => {
      setTxStage(event.stage);
      if (event.stage === "confirmed") {
        confirmedMilestonesRef.current = predicted;
        setTxError(null);
      }
      if (event.stage === "failed") {
        setTxError(event.error ?? "Transaction failed");
        setMilestones([...confirmedMilestonesRef.current]);
      }
    });

    setTxStage(null);
    setTxError(null);
    return tracker;
  }, []);

  const setConfirmed = useCallback((freshGrant: Grant, freshMilestones?: Milestone[]) => {
    grantStoreRef.current.setConfirmed(freshGrant);
    if (freshMilestones !== undefined) {
      confirmedMilestonesRef.current = freshMilestones;
      setMilestones(freshMilestones);
    }
    setTxStage(null);
    setTxError(null);
  }, []);

  const rollback = useCallback(() => {
    grantStoreRef.current.rollback();
    setMilestones([...confirmedMilestonesRef.current]);
    setTxStage("failed");
    setTxError("Manually rolled back");
  }, []);

  return { grant, milestones, isPending, txStage, txError, applyMutation, setConfirmed, rollback };
}
