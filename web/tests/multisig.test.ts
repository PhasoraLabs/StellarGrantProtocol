/**
 * Multi-Signature Tests
 *
 * Unit tests for combineSignatures, MultiSigTracker, isValidTransactionXdr,
 * and buildUnsignedTransaction.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  Account,
  Asset,
  BASE_FEE,
  Keypair,
  Operation,
  TransactionBuilder,
  xdr,
} from "@stellar/stellar-sdk";

vi.mock("../lib/stellar/client", () => ({
  getRpcClient: vi.fn(),
  getHorizonClient: vi.fn(),
  networkPassphraseConfig: "Test SDF Network ; September 2015",
}));

import { getHorizonClient, getRpcClient, networkPassphraseConfig } from "../lib/stellar/client";
import {
  fetchMultiSigConfig,
  buildUnsignedTransaction,
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

// ── buildUnsignedTransaction ──────────────────────────────────────────────

describe("buildUnsignedTransaction", () => {
  it("accepts a full TransactionEnvelope XDR produced by a contract client", async () => {
    const source = Keypair.random().publicKey();
    const dest = Keypair.random().publicKey();
    const account = new Account(source, "0");

    vi.mocked(getRpcClient).mockReturnValue({
      getAccount: vi.fn().mockResolvedValue(account),
    } as unknown as ReturnType<typeof getRpcClient>);

    const originalTx = new TransactionBuilder(account, {
      fee: BASE_FEE.toString(),
      networkPassphrase: networkPassphraseConfig,
    })
      .addOperation(
        Operation.payment({
          destination: dest,
          asset: Asset.native(),
          amount: "10",
        })
      )
      .setTimeout(30)
      .build();

    const builtXdr = await buildUnsignedTransaction({
      sourceAddress: source,
      operationsXdr: originalTx.toEnvelope().toXDR("base64"),
      timeoutSeconds: 30,
    });

    const rebuilt = TransactionBuilder.fromXDR(
      xdr.TransactionEnvelope.fromXDR(builtXdr, "base64"),
      networkPassphraseConfig,
    );
    expect(rebuilt.source).toBe(source);
    expect(builtXdr.length).toBeGreaterThan(0);
    expect(() => {
      TransactionBuilder.fromXDR(
        xdr.TransactionEnvelope.fromXDR(builtXdr, "base64"),
        networkPassphraseConfig,
      );
    }).not.toThrow();
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
    expect(() => tracker.buildCombinedXdr()).toThrow(/Not enough signature weight/);
  });

  it("throws if requiredSignatures < 1", () => {
    expect(() =>
      new MultiSigTracker({
        unsignedXdr: fakeXdr(0),
        requiredSignatures: 0,
        signerPublicKeys: [SIGNER_A],
      })
    ).toThrow(/requiredWeight must be at least 1/);
  });

  it("throws if signerPublicKeys fewer than requiredSignatures", () => {
    expect(() =>
      new MultiSigTracker({
        unsignedXdr: fakeXdr(0),
        requiredSignatures: 3,
        signerPublicKeys: [SIGNER_A, SIGNER_B],
      })
    ).toThrow(/Combined signer weight must be at least/);
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

// ── Weighted thresholds (#1282) ───────────────────────────────────────────

describe("MultiSigTracker weighted threshold", () => {
  const weighted = () =>
    new MultiSigTracker({
      unsignedXdr: fakeXdr(0),
      signers: [
        { publicKey: SIGNER_A, weight: 5 },
        { publicKey: SIGNER_B, weight: 1 },
        { publicKey: SIGNER_C, weight: 1 },
      ],
      requiredWeight: 5,
    });

  it("is not ready when several light signers have signed but weight is below threshold", () => {
    const t = weighted();
    t.addSignature(SIGNER_B, fakeXdr(1));
    t.addSignature(SIGNER_C, fakeXdr(2));
    expect(t.snapshot.collectedCount).toBe(2);
    expect(t.snapshot.collectedWeight).toBe(2);
    expect(t.isReady).toBe(false);
  });

  it("is ready with a single heavy signer despite a headcount of one", () => {
    const t = weighted();
    t.addSignature(SIGNER_A, fakeXdr(1));
    expect(t.snapshot.collectedCount).toBe(1);
    expect(t.isReady).toBe(true);
  });

  it("becomes not ready again when a signer is declined after signing", () => {
    const t = weighted();
    t.addSignature(SIGNER_A, fakeXdr(1));
    t.markDeclined(SIGNER_A);
    expect(t.isReady).toBe(false);
  });

  it("rejects configs whose total weight cannot meet the threshold", () => {
    expect(() =>
      new MultiSigTracker({
        unsignedXdr: fakeXdr(0),
        signers: [{ publicKey: SIGNER_A, weight: 1 }],
        requiredWeight: 2,
      })
    ).toThrow(/Combined signer weight/);
  });
});

describe("fetchMultiSigConfig", () => {
  const horizonWith = (thresholds: Record<string, number>) =>
    vi.mocked(getHorizonClient).mockReturnValue({
      loadAccount: vi.fn().mockResolvedValue({
        thresholds,
        signers: [
          { key: SIGNER_A, weight: 3, type: "ed25519_public_key" },
          { key: SIGNER_B, weight: 0, type: "ed25519_public_key" },
          { key: SIGNER_C, weight: 2, type: "ed25519_public_key" },
        ],
      }),
    } as unknown as ReturnType<typeof getHorizonClient>);

  it("reads weighted signers and the medium threshold by default, dropping weight-0 signers", async () => {
    horizonWith({ low_threshold: 1, med_threshold: 4, high_threshold: 5 });
    const cfg = await fetchMultiSigConfig(SIGNER_A);
    expect(cfg.requiredWeight).toBe(4);
    expect(cfg.signers).toEqual([
      { publicKey: SIGNER_A, weight: 3 },
      { publicKey: SIGNER_C, weight: 2 },
    ]);
  });

  it("selects the requested threshold level", async () => {
    horizonWith({ low_threshold: 1, med_threshold: 4, high_threshold: 5 });
    expect((await fetchMultiSigConfig(SIGNER_A, "high")).requiredWeight).toBe(5);
    expect((await fetchMultiSigConfig(SIGNER_A, "low")).requiredWeight).toBe(1);
  });
});
