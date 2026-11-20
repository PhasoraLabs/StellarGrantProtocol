/**
 * Optimistic UI State Management Tests
 */

import { describe, it, expect, vi } from "vitest";
import {
  TransactionTracker,
  OptimisticStore,
  predictGrantState,
  createOptimisticGrantMutation,
  type GrantMutation,
} from "../lib/utils/optimistic";
import type { Grant, Milestone } from "../types";

// ── Fixtures ──────────────────────────────────────────────────────────────

const baseGrant: Grant = {
  id: "1",
  owner: "GABC123",
  title: "Test Grant",
  description: "A test grant",
  budget: 1_000_000_000n,
  funded: 0n,
  deadline: 9_999_999_999n,
  status: 1,
  milestones: 2,
  reviewers: [],
  created_at: 1_000_000n,
};

const baseMilestone: Milestone = {
  idx: 0,
  title: "M1",
  description: "First milestone",
  proof_hash: null,
  submitted: false,
  approved: false,
  paid: false,
  submitted_at: null,
  approved_at: null,
  paid_at: null,
};

// ── TransactionTracker ────────────────────────────────────────────────────

describe("TransactionTracker", () => {
  it("emits events in order", () => {
    const tracker = new TransactionTracker();
    const stages: string[] = [];
    tracker.on((e) => stages.push(e.stage));

    tracker.markSigned();
    tracker.markSubmitted("tx1");
    tracker.markConfirmed("tx1");

    expect(stages).toEqual(["signed", "submitted", "confirmed"]);
  });

  it("passes txHash through submitted and confirmed events", () => {
    const tracker = new TransactionTracker();
    const hashes: (string | undefined)[] = [];
    tracker.on((e) => hashes.push(e.txHash));

    tracker.markSigned();
    tracker.markSubmitted("abc123");
    tracker.markConfirmed("abc123");

    expect(hashes).toEqual([undefined, "abc123", "abc123"]);
  });

  it("emits failed event with error message", () => {
    const tracker = new TransactionTracker();
    let errorMsg = "";
    tracker.on((e) => { if (e.stage === "failed") errorMsg = e.error ?? ""; });

    tracker.markFailed("insufficient balance");
    expect(errorMsg).toBe("insufficient balance");
  });

  it("isTerminal is false until confirmed or failed", () => {
    const tracker = new TransactionTracker();
    expect(tracker.isTerminal).toBe(false);
    tracker.markSigned();
    expect(tracker.isTerminal).toBe(false);
    tracker.markSubmitted("tx");
    expect(tracker.isTerminal).toBe(false);
    tracker.markConfirmed("tx");
    expect(tracker.isTerminal).toBe(true);
  });

  it("off() removes a listener", () => {
    const tracker = new TransactionTracker();
    const fn = vi.fn();
    tracker.on(fn);
    tracker.off(fn);
    tracker.markSigned();
    expect(fn).not.toHaveBeenCalled();
  });

  it("destroy() removes all listeners", () => {
    const tracker = new TransactionTracker();
    const fn = vi.fn();
    tracker.on(fn);
    tracker.destroy();
    tracker.markSigned();
    expect(fn).not.toHaveBeenCalled();
  });

  it("returns unsubscribe function from on()", () => {
    const tracker = new TransactionTracker();
    const fn = vi.fn();
    const unsubscribe = tracker.on(fn);
    unsubscribe();
    tracker.markSigned();
    expect(fn).not.toHaveBeenCalled();
  });

  it("includes meta in events", () => {
    const tracker = new TransactionTracker({ meta: { grantId: "42" } });
    let meta: Record<string, unknown> | undefined;
    tracker.on((e) => { meta = e.meta; });
    tracker.markSigned();
    expect(meta?.grantId).toBe("42");
  });
});

// ── predictGrantState ─────────────────────────────────────────────────────

describe("predictGrantState", () => {
  it("fund: increases funded amount", () => {
    const mutation: GrantMutation = { type: "fund", amount: 500_000_000n };
    const { grant } = predictGrantState(baseGrant, mutation);
    expect(grant.funded).toBe(500_000_000n);
  });

  it("fund: upgrades status when fully funded", () => {
    const mutation: GrantMutation = { type: "fund", amount: 1_000_000_000n };
    const { grant } = predictGrantState(baseGrant, mutation);
    expect(grant.status).toBe(2);
  });

  it("fund: does not downgrade existing higher status", () => {
    const active: Grant = { ...baseGrant, status: 3 };
    const mutation: GrantMutation = { type: "fund", amount: 1_000_000_000n };
    const { grant } = predictGrantState(active, mutation);
    expect(grant.status).toBe(3);
  });

  it("statusChange: updates grant status", () => {
    const mutation: GrantMutation = { type: "statusChange", newStatus: 4 };
    const { grant } = predictGrantState(baseGrant, mutation);
    expect(grant.status).toBe(4);
  });

  it("milestoneSubmit: marks milestone as submitted", () => {
    const mutation: GrantMutation = { type: "milestoneSubmit", milestoneIdx: 0, proofHash: "hash123" };
    const { milestones } = predictGrantState(baseGrant, mutation, [baseMilestone]);
    expect(milestones[0].submitted).toBe(true);
    expect(milestones[0].proof_hash).toBe("hash123");
    expect(milestones[0].submitted_at).not.toBeNull();
  });

  it("milestoneApprove: marks milestone as approved", () => {
    const submitted = { ...baseMilestone, submitted: true, proof_hash: "hash123" };
    const mutation: GrantMutation = { type: "milestoneApprove", milestoneIdx: 0 };
    const { milestones } = predictGrantState(baseGrant, mutation, [submitted]);
    expect(milestones[0].approved).toBe(true);
    expect(milestones[0].approved_at).not.toBeNull();
  });

  it("milestoneReject: rolls back submission", () => {
    const submitted = { ...baseMilestone, submitted: true, proof_hash: "hash123", submitted_at: 999n };
    const mutation: GrantMutation = { type: "milestoneReject", milestoneIdx: 0 };
    const { milestones } = predictGrantState(baseGrant, mutation, [submitted]);
    expect(milestones[0].submitted).toBe(false);
    expect(milestones[0].proof_hash).toBeNull();
    expect(milestones[0].submitted_at).toBeNull();
  });

  it("does not mutate the original grant", () => {
    const original = { ...baseGrant };
    const mutation: GrantMutation = { type: "fund", amount: 100n };
    predictGrantState(baseGrant, mutation);
    expect(baseGrant.funded).toBe(original.funded);
  });

  it("vote: adds an approval vote entry for the reviewer", () => {
    const mutation: GrantMutation = { type: "vote", milestoneIdx: 0, reviewer: "GREVIEWER", approve: true };
    const { milestones } = predictGrantState(baseGrant, mutation, [baseMilestone]);
    expect(milestones[0].votes).toHaveLength(1);
    expect(milestones[0].votes?.[0]).toMatchObject({ reviewer: "GREVIEWER", vote: "approve" });
  });

  it("vote: records a rejection vote", () => {
    const mutation: GrantMutation = { type: "vote", milestoneIdx: 0, reviewer: "GREVIEWER", approve: false };
    const { milestones } = predictGrantState(baseGrant, mutation, [baseMilestone]);
    expect(milestones[0].votes?.[0].vote).toBe("reject");
  });

  it("vote: re-voting replaces the reviewer's prior vote (no duplicates)", () => {
    const withVote: Milestone = {
      ...baseMilestone,
      votes: [{ reviewer: "GREVIEWER", vote: "reject", voted_at: 1n }],
    };
    const mutation: GrantMutation = { type: "vote", milestoneIdx: 0, reviewer: "GREVIEWER", approve: true };
    const { milestones } = predictGrantState(baseGrant, mutation, [withVote]);
    expect(milestones[0].votes).toHaveLength(1);
    expect(milestones[0].votes?.[0].vote).toBe("approve");
  });

  it("vote: only touches the target milestone", () => {
    const m2: Milestone = { ...baseMilestone, idx: 1 };
    const mutation: GrantMutation = { type: "vote", milestoneIdx: 0, reviewer: "GREVIEWER", approve: true };
    const { milestones } = predictGrantState(baseGrant, mutation, [baseMilestone, m2]);
    expect(milestones[0].votes).toHaveLength(1);
    expect(milestones[1].votes).toBeUndefined();
  });
});

// ── OptimisticStore ───────────────────────────────────────────────────────

describe("OptimisticStore", () => {
  it("apply() updates optimistic state", () => {
    const store = new OptimisticStore(baseGrant);
    store.apply((g) => ({ ...g, funded: 500n }));
    expect(store.state.funded).toBe(500n);
    expect(store.confirmedState.funded).toBe(0n);
  });

  it("commit() locks in optimistic state as confirmed", () => {
    const store = new OptimisticStore(baseGrant);
    store.apply((g) => ({ ...g, funded: 500n }));
    store.commit();
    expect(store.confirmedState.funded).toBe(500n);
  });

  it("rollback() reverts to confirmed state", () => {
    const store = new OptimisticStore(baseGrant);
    store.apply((g) => ({ ...g, funded: 500n }));
    store.rollback();
    expect(store.state.funded).toBe(0n);
  });

  it("subscribe() notifies on apply", () => {
    const store = new OptimisticStore(baseGrant);
    const fn = vi.fn();
    store.subscribe(fn);
    store.apply((g) => ({ ...g, funded: 1n }));
    expect(fn).toHaveBeenCalledOnce();
  });

  it("unsubscribe stops notifications", () => {
    const store = new OptimisticStore(baseGrant);
    const fn = vi.fn();
    const unsub = store.subscribe(fn);
    unsub();
    store.apply((g) => ({ ...g, funded: 1n }));
    expect(fn).not.toHaveBeenCalled();
  });

  it("setConfirmed() syncs both states", () => {
    const store = new OptimisticStore(baseGrant);
    const fresh: Grant = { ...baseGrant, funded: 999n };
    store.setConfirmed(fresh);
    expect(store.state.funded).toBe(999n);
    expect(store.confirmedState.funded).toBe(999n);
  });
});

// ── createOptimisticGrantMutation ─────────────────────────────────────────

describe("createOptimisticGrantMutation", () => {
  it("applies optimistic state immediately", () => {
    const store = new OptimisticStore(baseGrant);
    createOptimisticGrantMutation(store, { type: "fund", amount: 200n });
    expect(store.state.funded).toBe(200n);
  });

  it("commits on confirmed", () => {
    const store = new OptimisticStore(baseGrant);
    const tracker = createOptimisticGrantMutation(store, { type: "fund", amount: 200n });
    tracker.markConfirmed("tx1");
    expect(store.confirmedState.funded).toBe(200n);
  });

  it("rolls back on failed", () => {
    const store = new OptimisticStore(baseGrant);
    const tracker = createOptimisticGrantMutation(store, { type: "fund", amount: 200n });
    tracker.markFailed("out of gas");
    expect(store.state.funded).toBe(0n);
  });
});
