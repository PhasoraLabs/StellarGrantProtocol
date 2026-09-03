/**
 * Multi-Signature Transaction Support
 *
 * Utilities for building, coordinating, and submitting Stellar transactions
 * that require multiple signers (multi-sig accounts).
 *
 * Workflow:
 * 1. Build an unsigned XDR with `buildUnsignedTransaction()`
 * 2. Distribute the XDR to each signer out-of-band
 * 3. Each signer calls `signTransactionXdr()` and returns their signed XDR
 * 4. Collect all signed XDRs and combine with `combineSignatures()`
 * 5. Track collection progress with `MultiSigTracker`
 * 6. Submit the combined XDR with `submitSignedXdr()`
 */

import {
  Transaction,
  TransactionBuilder,
  FeeBumpTransaction,
  xdr,
} from "@stellar/stellar-sdk";
import { rpc as StellarRpc } from "@stellar/stellar-sdk";
import { getRpcClient, networkPassphraseConfig } from "./client";

// ── Types ─────────────────────────────────────────────────────────────────

/** A raw base64-encoded XDR string representing a Stellar transaction */
export type TransactionXdr = string;

/** The status of a signer's contribution to a pending multi-sig transaction */
export type SignerStatus = "pending" | "signed" | "declined";

/** Describes a signer and their current signing status */
export interface SignerEntry {
  /** Stellar public key of the required signer */
  publicKey: string;
  /** Current status for this signer */
  status: SignerStatus;
  /** The signed XDR returned by this signer (present when status === "signed") */
  signedXdr?: TransactionXdr;
  /** ISO timestamp of when the signature was received */
  signedAt?: string;
}

/** Full snapshot of a pending multi-sig transaction's signature collection */
export interface MultiSigStatus {
  /** Unique ID for this multi-sig session */
  id: string;
  /** The unsigned base transaction XDR to be signed by each party */
  unsignedXdr: TransactionXdr;
  /** The minimum number of signatures required to meet the account threshold */
  requiredSignatures: number;
  /** All signers and their current status */
  signers: SignerEntry[];
  /** How many signers have provided a valid signature so far */
  collectedCount: number;
  /** True when enough signatures have been collected to submit */
  isReady: boolean;
  /** UTC timestamp when this session was created */
  createdAt: string;
}

/** Options for building an unsigned transaction XDR */
export interface BuildUnsignedTxOptions {
  /** The source account address (must be a funded account) */
  sourceAddress: string;
  /** Pre-built XDR operation(s) to include (e.g. from a contract simulation) */
  operationsXdr: TransactionXdr;
  /** Base fee in stroops (default: 100) */
  fee?: number;
  /** Transaction timeout in seconds (default: 30) */
  timeoutSeconds?: number;
}

/** Options for submitting a combined signed XDR */
export interface SubmitOptions {
  /** If true, wait for the transaction to be confirmed before returning */
  waitForConfirmation?: boolean;
}

// ── XDR Utilities ─────────────────────────────────────────────────────────

/**
 * Build an unsigned transaction XDR from a pre-simulated operation.
 *
 * Returns the XDR string without submitting it — suitable for distribution
 * to multiple signers in a multi-sig workflow.
 *
 * @param options - Source account, operations, fee, and timeout
 * @returns Base64-encoded unsigned transaction XDR
 */
export async function buildUnsignedTransaction(
  options: BuildUnsignedTxOptions
): Promise<TransactionXdr> {
  const { sourceAddress, operationsXdr, fee = 100, timeoutSeconds = 30 } = options;
  const rpc = getRpcClient();

  // Load the source account to get the current sequence number
  const account = await rpc.getAccount(sourceAddress);

  // Deserialize the operation from XDR
  const operation = xdr.Operation.fromXDR(operationsXdr, "base64");

  const tx = new TransactionBuilder(account, {
    fee: fee.toString(),
    networkPassphrase: networkPassphraseConfig,
  })
    .addOperation(operation)
    .setTimeout(timeoutSeconds)
    .build();

  return tx.toXDR();
}

/**
 * Combine multiple signed XDRs into a single transaction XDR.
 *
 * Each signed XDR must derive from the same unsigned base transaction.
 * Signatures from all XDRs are merged into the first XDR, producing a
 * single transaction that satisfies the multi-sig threshold.
 *
 * @param baseXdr     - The original unsigned (or partially signed) XDR
 * @param signedXdrs  - Array of XDRs each containing one signer's signature
 * @returns A merged XDR containing all signatures
 * @throws If any XDR cannot be parsed or contains a mismatched transaction hash
 */
export function combineSignatures(
  baseXdr: TransactionXdr,
  signedXdrs: TransactionXdr[]
): TransactionXdr {
  if (signedXdrs.length === 0) {
    return baseXdr;
  }

  // Parse the base transaction
  const base = parseTransaction(baseXdr);
  const baseHash = base.hash();

  for (const signedXdr of signedXdrs) {
    const signed = parseTransaction(signedXdr);

    // Verify both transactions have the same hash (same envelope content)
    const signedHash = signed.hash();
    if (!baseHash.equals(signedHash)) {
      throw new Error(
        "combineSignatures: mismatched transaction hash — all XDRs must be based on the same transaction"
      );
    }

    // Merge each decorated signature from the signed tx into the base tx
    for (const sig of signed.signatures) {
      // Avoid duplicates — skip if already present
      const alreadyPresent = base.signatures.some(
        (existing) =>
          existing.signature().equals(sig.signature()) &&
          existing.hint().equals(sig.hint())
      );
      if (!alreadyPresent) {
        base.addDecoratedSignature(sig);
      }
    }
  }

  return base.toXDR();
}

/**
 * Submit a fully-signed XDR to the Stellar network.
 *
 * @param signedXdr - The combined, fully-signed transaction XDR
 * @param options   - Submission options (waitForConfirmation, etc.)
 * @returns The transaction hash on success
 * @throws On network error or transaction rejection
 */
export async function submitSignedXdr(
  signedXdr: TransactionXdr,
  options: SubmitOptions = {}
): Promise<{ txHash: string; ledger?: number }> {
  const { waitForConfirmation = true } = options;
  const rpc = getRpcClient();

  const tx = parseTransaction(signedXdr);
  const response = await rpc.sendTransaction(tx);

  if (response.status === "ERROR") {
    throw new Error(
      `Transaction submission failed: ${response.errorResult?.result().value() ?? "unknown error"}`
    );
  }

  const txHash = response.hash;

  if (!waitForConfirmation) {
    return { txHash };
  }

  // Poll for confirmation
  const confirmation = await pollForConfirmation(rpc, txHash);
  return { txHash, ledger: confirmation.ledger };
}

// ── MultiSigTracker ───────────────────────────────────────────────────────

/**
 * Tracks the out-of-band signature collection process for a multi-sig
 * transaction. Framework-agnostic — works with React, Vue, or plain JS.
 *
 * @example
 * ```ts
 * const tracker = new MultiSigTracker({
 *   unsignedXdr,
 *   requiredSignatures: 2,
 *   signerPublicKeys: ["GAAA...", "GBBB...", "GCCC..."],
 * });
 *
 * // When a signer returns their signed XDR:
 * tracker.addSignature("GAAA...", signedXdrFromGAAA);
 *
 * if (tracker.isReady) {
 *   const combinedXdr = tracker.buildCombinedXdr();
 *   await submitSignedXdr(combinedXdr);
 * }
 * ```
 */
export class MultiSigTracker {
  private status: MultiSigStatus;
  private listeners: Set<(status: MultiSigStatus) => void> = new Set();

  constructor(options: {
    unsignedXdr: TransactionXdr;
    requiredSignatures: number;
    signerPublicKeys: string[];
    id?: string;
  }) {
    const { unsignedXdr, requiredSignatures, signerPublicKeys, id } = options;

    if (requiredSignatures < 1) {
      throw new Error("requiredSignatures must be at least 1");
    }
    if (signerPublicKeys.length < requiredSignatures) {
      throw new Error(
        "signerPublicKeys must contain at least requiredSignatures entries"
      );
    }

    this.status = {
      id: id ?? crypto.randomUUID(),
      unsignedXdr,
      requiredSignatures,
      signers: signerPublicKeys.map((pk) => ({ publicKey: pk, status: "pending" })),
      collectedCount: 0,
      isReady: false,
      createdAt: new Date().toISOString(),
    };
  }

  /** Current status snapshot */
  get snapshot(): MultiSigStatus {
    return { ...this.status, signers: this.status.signers.map((s) => ({ ...s })) };
  }

  /** True when enough signatures have been collected */
  get isReady(): boolean {
    return this.status.isReady;
  }

  /** Register a listener for status changes. Returns unsubscribe fn. */
  onChange(listener: (status: MultiSigStatus) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /**
   * Record a signer's signed XDR.
   *
   * @param signerPublicKey - The public key of the signer
   * @param signedXdr       - The XDR returned by this signer's wallet
   * @throws If the public key is not in the signer list
   */
  addSignature(signerPublicKey: string, signedXdr: TransactionXdr): void {
    const entry = this.status.signers.find((s) => s.publicKey === signerPublicKey);
    if (!entry) {
      throw new Error(`Unknown signer: ${signerPublicKey}`);
    }

    entry.status = "signed";
    entry.signedXdr = signedXdr;
    entry.signedAt = new Date().toISOString();

    this.recompute();
    this.notify();
  }

  /**
   * Record that a signer has declined to sign.
   *
   * @param signerPublicKey - The public key of the declining signer
   */
  markDeclined(signerPublicKey: string): void {
    const entry = this.status.signers.find((s) => s.publicKey === signerPublicKey);
    if (!entry) {
      throw new Error(`Unknown signer: ${signerPublicKey}`);
    }
    entry.status = "declined";
    this.recompute();
    this.notify();
  }

  /**
   * Build a combined XDR from all collected signatures.
   * Only callable when `isReady === true`.
   *
   * @throws If not enough signatures have been collected yet
   */
  buildCombinedXdr(): TransactionXdr {
    if (!this.status.isReady) {
      throw new Error(
        `Not enough signatures: have ${this.status.collectedCount}, need ${this.status.requiredSignatures}`
      );
    }

    const signedXdrs = this.status.signers
      .filter((s) => s.status === "signed" && s.signedXdr)
      .map((s) => s.signedXdr!);

    return combineSignatures(this.status.unsignedXdr, signedXdrs);
  }

  /** Pending signers (have not yet signed or declined) */
  get pendingSigners(): SignerEntry[] {
    return this.status.signers.filter((s) => s.status === "pending");
  }

  private recompute(): void {
    const collected = this.status.signers.filter((s) => s.status === "signed").length;
    this.status.collectedCount = collected;
    this.status.isReady = collected >= this.status.requiredSignatures;
  }

  private notify(): void {
    const snap = this.snapshot;
    this.listeners.forEach((l) => l(snap));
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────

/**
 * Parse a base64 XDR string into a Transaction or FeeBumpTransaction.
 * @throws If the XDR is invalid or malformed.
 */
function parseTransaction(xdrString: TransactionXdr): Transaction {
  try {
    const envelope = xdr.TransactionEnvelope.fromXDR(xdrString, "base64");
    const tx = TransactionBuilder.fromXDR(envelope, networkPassphraseConfig);
    if (tx instanceof FeeBumpTransaction) {
      throw new Error("FeeBumpTransaction is not supported in multi-sig mode");
    }
    return tx;
  } catch (err) {
    if (err instanceof Error && err.message.includes("FeeBumpTransaction")) throw err;
    throw new Error(`Invalid transaction XDR: ${err instanceof Error ? err.message : String(err)}`);
  }
}

const POLL_INTERVAL_MS = 3_000;
const POLL_MAX_ATTEMPTS = 20;

async function pollForConfirmation(
  rpc: StellarRpc.Server,
  txHash: string
): Promise<{ ledger: number }> {
  for (let attempt = 0; attempt < POLL_MAX_ATTEMPTS; attempt++) {
    const result = await rpc.getTransaction(txHash);

    if (result.status === StellarRpc.Api.GetTransactionStatus.SUCCESS) {
      return { ledger: result.ledger ?? 0 };
    }

    if (result.status === StellarRpc.Api.GetTransactionStatus.FAILED) {
      throw new Error(`Transaction failed on-chain: ${txHash}`);
    }

    // NOT_FOUND means still pending — wait and retry
    await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS));
  }

  throw new Error(
    `Transaction not confirmed after ${POLL_MAX_ATTEMPTS * POLL_INTERVAL_MS / 1000}s: ${txHash}`
  );
}

/**
 * Convenience: verify that a given XDR string is a valid, parseable transaction.
 * Useful for validating signer-submitted XDRs before merging.
 *
 * @returns true if valid, false otherwise
 */
export function isValidTransactionXdr(xdrString: TransactionXdr): boolean {
  try {
    parseTransaction(xdrString);
    return true;
  } catch {
    return false;
  }
}
