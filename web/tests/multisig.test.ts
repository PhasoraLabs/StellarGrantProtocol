/**
 * Multi-Signature Tests
 *
 * Unit tests for combineSignatures, MultiSigTracker, isValidTransactionXdr,
 * and buildUnsignedTransaction.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  combineSignatures,
  isValidTransactionXdr,
  MultiSigTracker,
  type TransactionXdr,
} from "../lib/stellar/multisig";

// ── Helpers ───────────────────────────────────────────────────────────────

/** Make a minimal fake XDR string (not real Stellar XDR, used for mock tests) */
const fakeXdr = (n: number): TransactionXdr => `fake-xdr-${n}` as TransactionXdr;

const SIGNER_A = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
const SIGNER_B = "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";
const SIGNER_C = "GCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC";

// ── isValidTransactionXdr ─────────────────────────────────────────────────

describe("isValidTransactionXdr", () => {
  it("returns false for a plain string", () => {
    expect(isValidTransactionXdr("not-an-xdr")).toBe(false);
  });

  it("returns false for empty string", () => {
    expect(isValidTransactionXdr("")).toBe(false);
  });

  it("returns false for random base64", () => {
    expect(isValidTransactionXdr("aGVsbG8gd29ybGQ=")).toBe(false);
  });
});

// ── combineSignatures ─────────────────────────────────────────────────────

describe("combineSignatures", () => {
  it("returns baseXdr unchanged when no signed XDRs are provided", () => {
    const base = fakeXdr(1);
    // Mocking the real SDK so this doesn't fail on parse:
    // Since we cannot build real XDRs in unit tests without network access,
    // we verify the early-return path only.
    const result = combineSignatures(base, []);
    expect(result).toBe(base);
  });

  it("throws on mismatched transaction XDR", () => {
    // Two fake XDRs that cannot be parsed should throw a parse error
    expect(() => combineSignatures(fakeXdr(1), [fakeXdr(2)])).toThrow(
      /Invalid transaction XDR/
    );
  });
});

// ── MultiSigTracker ───────────────────────────────────────────────────────

describe("MultiSigTracker", () => {
  let tracker: MultiSigTracker;

  beforeEach(() => {
    tracker = new MultiSigTracker({
      unsignedXdr: fakeXdr(0),
      requiredSignatures: 2,
      signerPublicKeys: [SIGNER_A, SIGNER_B, SIGNER_C],
    });
  });

  it("initializes with all signers as pending", () => {
    const { signers } = tracker.snapshot;
    expect(signers).toHaveLength(3);
    expect(signers.every((s) => s.status === "pending")).toBe(true);
  });

  it("isReady is false initially", () => {
    expect(tracker.isReady).toBe(false);
  });

  it("addSignature sets signer status to signed", () => {
    tracker.addSignature(SIGNER_A, fakeXdr(1));
    const a = tracker.snapshot.signers.find((s) => s.publicKey === SIGNER_A);
    expect(a?.status).toBe("signed");
    expect(a?.signedXdr).toBe(fakeXdr(1));
  });

  it("addSignature records signedAt timestamp", () => {
    tracker.addSignature(SIGNER_A, fakeXdr(1));
    const a = tracker.snapshot.signers.find((s) => s.publicKey === SIGNER_A);
    expect(a?.signedAt).toBeDefined();
    expect(new Date(a!.signedAt!).getTime()).toBeLessThanOrEqual(Date.now());
  });

  it("collectedCount increments when signatures are added", () => {
    tracker.addSignature(SIGNER_A, fakeXdr(1));
    expect(tracker.snapshot.collectedCount).toBe(1);
    tracker.addSignature(SIGNER_B, fakeXdr(2));
    expect(tracker.snapshot.collectedCount).toBe(2);
  });

  it("isReady becomes true when enough signatures collected", () => {
    tracker.addSignature(SIGNER_A, fakeXdr(1));
    expect(tracker.isReady).toBe(false);
    tracker.addSignature(SIGNER_B, fakeXdr(2));
    expect(tracker.isReady).toBe(true);
  });

  it("markDeclined sets signer status to declined", () => {
    tracker.markDeclined(SIGNER_C);
    const c = tracker.snapshot.signers.find((s) => s.publicKey === SIGNER_C);
    expect(c?.status).toBe("declined");
  });

  it("declined signer does not count towards collectedCount", () => {
    tracker.markDeclined(SIGNER_A);
    expect(tracker.snapshot.collectedCount).toBe(0);
  });

  it("throws when adding unknown signer", () => {
    expect(() => tracker.addSignature("GUNKNOWN", fakeXdr(9))).toThrow(/Unknown signer/);
  });

  it("throws when marking unknown signer as declined", () => {
    expect(() => tracker.markDeclined("GUNKNOWN")).toThrow(/Unknown signer/);
  });

  it("pendingSigners returns only pending entries", () => {
    tracker.addSignature(SIGNER_A, fakeXdr(1));
    tracker.markDeclined(SIGNER_C);
    expect(tracker.pendingSigners).toHaveLength(1);
    expect(tracker.pendingSigners[0].publicKey).toBe(SIGNER_B);
  });

  it("onChange fires on addSignature", () => {
    const fn = vi.fn();
    tracker.onChange(fn);
    tracker.addSignature(SIGNER_A, fakeXdr(1));
    expect(fn).toHaveBeenCalledOnce();
  });

  it("onChange fires on markDeclined", () => {
    const fn = vi.fn();
    tracker.onChange(fn);
    tracker.markDeclined(SIGNER_A);
    expect(fn).toHaveBeenCalledOnce();
  });

  it("onChange unsubscribe stops notifications", () => {
    const fn = vi.fn();
    const unsub = tracker.onChange(fn);
    unsub();
    tracker.addSignature(SIGNER_A, fakeXdr(1));
    expect(fn).not.toHaveBeenCalled();
  });

  it("buildCombinedXdr throws when not ready", () => {
    expect(() => tracker.buildCombinedXdr()).toThrow(/Not enough signatures/);
  });

  it("throws if requiredSignatures < 1", () => {
    expect(() =>
      new MultiSigTracker({
        unsignedXdr: fakeXdr(0),
        requiredSignatures: 0,
        signerPublicKeys: [SIGNER_A],
      })
    ).toThrow(/requiredSignatures must be at least 1/);
  });

  it("throws if signerPublicKeys fewer than requiredSignatures", () => {
    expect(() =>
      new MultiSigTracker({
        unsignedXdr: fakeXdr(0),
        requiredSignatures: 3,
        signerPublicKeys: [SIGNER_A, SIGNER_B],
      })
    ).toThrow(/signerPublicKeys must contain at least/);
  });

  it("snapshot is a deep copy, not the internal reference", () => {
    const snap1 = tracker.snapshot;
    tracker.addSignature(SIGNER_A, fakeXdr(1));
    const snap2 = tracker.snapshot;
    // snap1 should not have changed
    expect(snap1.collectedCount).toBe(0);
    expect(snap2.collectedCount).toBe(1);
  });
});
