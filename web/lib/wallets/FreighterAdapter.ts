/**
 * Freighter wallet adapter.
 *
 * Wraps @stellar/freighter-api and normalises every way Freighter can fail —
 * extension not installed, wallet locked, request rejected, wrong network —
 * into a single FreighterWalletError carrying a stable `code` plus an
 * actionable message, so the UI can tell the user exactly what to do next
 * instead of surfacing a generic message or "[object Object]".
 */

import {
  getAddress,
  getNetworkDetails,
  isAllowed,
  setAllowed,
  signTransaction as freighterSignTransaction,
} from "@stellar/freighter-api";
import {
  FeeBumpTransaction,
  Networks,
  TransactionBuilder,
  xdr,
} from "@stellar/stellar-sdk";
import type { WalletAdapter } from "./types";

/** Stable classification of a Freighter failure. */
export type FreighterWalletErrorCode =
  | "NOT_INSTALLED"
  | "LOCKED"
  | "REJECTED"
  | "NETWORK_MISMATCH"
  | "UNKNOWN";

/**
 * Error states that can be derived from a failure message. NETWORK_MISMATCH is
 * only ever set explicitly by signTransaction, never inferred from text.
 */
export type FreighterMessageErrorCode = Exclude<FreighterWalletErrorCode, "NETWORK_MISMATCH">;

/** How long to wait for the Freighter content script to inject `window.freighter`. */
export const DEFAULT_INJECTION_TIMEOUT_MS = 3000;

const INJECTION_POLL_INTERVAL_MS = 100;

/** Actionable, user-facing message for each error state. */
const ACTIONABLE_MESSAGES: Record<FreighterMessageErrorCode, string> = {
  NOT_INSTALLED:
    "Freighter is not available in this browser. Install or enable the Freighter extension (https://freighter.app), reload the page, and try again.",
  LOCKED:
    "Your Freighter wallet is locked. Open the Freighter extension, unlock it, and try again.",
  REJECTED:
    "The request was rejected in Freighter. Open the extension, approve the request, and try again.",
  UNKNOWN: "Freighter could not complete the request.",
};

/**
 * Error thrown by the Freighter adapter. `code` identifies the failure state
 * and `message` tells the user how to resolve it.
 */
export class FreighterWalletError extends Error {
  readonly code: FreighterWalletErrorCode;

  constructor(
    code: FreighterWalletErrorCode,
    message: string,
    options?: { cause?: unknown },
  ) {
    super(message, options?.cause !== undefined ? { cause: options.cause } : undefined);
    this.name = "FreighterWalletError";
    this.code = code;
  }
}

/** Pull a readable message out of the shapes Freighter can throw or return. */
function readErrorMessage(raw: unknown): string {
  if (typeof raw === "string") return raw.trim();
  if (raw instanceof Error) return raw.message.trim();
  if (raw !== null && typeof raw === "object") {
    const candidate = raw as { message?: unknown; error?: unknown };
    if (typeof candidate.message === "string" && candidate.message.trim()) {
      return candidate.message.trim();
    }
    if (typeof candidate.error === "string" && candidate.error.trim()) {
      return candidate.error.trim();
    }
  }
  return "";
}

/**
 * Map a raw failure message onto an error state. Ordered most-specific
 * first: a missing extension, then a locked wallet, then a user rejection.
 */
export function classifyFreighterMessage(message: string): FreighterMessageErrorCode {
  if (
    /\bnot installed\b|\bnot available\b|\bno freighter\b|\bis not defined\b|\bundefined\b|\bnot injected\b|\bcontext invalidated\b/i.test(
      message,
    )
  ) {
    return "NOT_INSTALLED";
  }
  if (/\b(un)?lock(ed|ing|s)?\b/i.test(message)) {
    return "LOCKED";
  }
  if (
    /\breject\w*|\bdenied\b|\bdeclin\w*|\bcancel\w*|\brefus\w*|\bdismiss\w*|\btimeout\b|\btimed out\b|\bexpired\b|\baborted?\b|\bnot allowed\b|\bpermission\b|\bclosed\b/i.test(
      message,
    )
  ) {
    return "REJECTED";
  }
  return "UNKNOWN";
}

/**
 * Normalise any thrown/rejected value into a FreighterWalletError with an
 * actionable message. Values that are already FreighterWalletErrors pass
 * through untouched so existing states are preserved.
 */
export function toFreighterWalletError(raw: unknown): FreighterWalletError {
  if (raw instanceof FreighterWalletError) return raw;

  const detail = readErrorMessage(raw);
  const code = classifyFreighterMessage(detail);

  const message =
    code === "UNKNOWN"
      ? detail
        ? `Freighter request failed: ${detail}`
        : ACTIONABLE_MESSAGES.UNKNOWN
      : ACTIONABLE_MESSAGES[code];

  return new FreighterWalletError(code, message, {
    cause: raw !== undefined && raw !== null ? raw : undefined,
  });
}

/** Human-readable label for a network passphrase, for error messages. */
function networkLabel(passphrase: string): string {
  if (passphrase === Networks.PUBLIC) return "Public";
  if (passphrase === Networks.TESTNET) return "Testnet";
  if (passphrase === Networks.FUTURENET) return "Futurenet";
  return passphrase;
}

function getInjectedFreighter(): unknown {
  if (typeof window === "undefined") return undefined;
  return (window as Window & { freighter?: unknown }).freighter;
}

/**
 * Poll until Freighter injects `window.freighter` (the content script can run
 * a beat after page load), or until `timeoutMs` elapses.
 *
 * @returns true once the extension is injected.
 */
export async function waitForFreighterInjection(
  timeoutMs: number = DEFAULT_INJECTION_TIMEOUT_MS,
): Promise<boolean> {
  let waitedMs = 0;
  while (waitedMs < timeoutMs) {
    if (getInjectedFreighter()) return true;
    await new Promise<void>((resolve) =>
      setTimeout(resolve, INJECTION_POLL_INTERVAL_MS),
    );
    waitedMs += INJECTION_POLL_INTERVAL_MS;
  }
  return Boolean(getInjectedFreighter());
}

/** Run a Freighter API call, classifying synchronous rejections. */
async function callFreighter<T>(call: () => Promise<T>): Promise<T> {
  try {
    return await call();
  } catch (error) {
    throw toFreighterWalletError(error);
  }
}

export function verifySignedTransactionMatchesOriginal(
  originalTxXdr: string,
  signedTxXdr: string,
  networkPassphrase: string,
): void {
  const parseTx = (name: string, txXdr: string) => {
    try {
      const envelope = xdr.TransactionEnvelope.fromXDR(txXdr, "base64");
      const tx = TransactionBuilder.fromXDR(envelope, networkPassphrase);
      if (tx instanceof FeeBumpTransaction) {
        throw new Error("FeeBumpTransaction is not supported for wallet signing validation");
      }
      return tx;
    } catch (error) {
      throw new Error(`Invalid ${name} transaction XDR: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const original = parseTx("original", originalTxXdr);
  const signed = parseTx("signed", signedTxXdr);

  if (!original.hash().equals(signed.hash())) {
    throw new Error("Signed transaction does not match the original transaction hash.");
  }

  if (original.source !== signed.source) {
    throw new Error("Signed transaction source account does not match the original transaction.");
  }

  if (original.sequence !== signed.sequence) {
    throw new Error("Signed transaction sequence number does not match the original transaction.");
  }
}

export class FreighterAdapter implements WalletAdapter {
  readonly name = "Freighter";
  readonly icon = "https://freighter.app/favicon.ico";

  /** How long to wait for `window.freighter` before reporting NOT_INSTALLED. */
  private readonly injectionTimeoutMs: number;

  constructor(options: { injectionTimeoutMs?: number } = {}) {
    this.injectionTimeoutMs = options.injectionTimeoutMs ?? DEFAULT_INJECTION_TIMEOUT_MS;
  }

  isAvailable(): boolean {
    return Boolean(getInjectedFreighter());
  }

  private async ensureInjected(): Promise<void> {
    if (typeof window === "undefined") {
      throw new FreighterWalletError(
        "NOT_INSTALLED",
        "Freighter is only available in the browser. Open this page in a browser with the Freighter extension installed.",
      );
    }
    if (await waitForFreighterInjection(this.injectionTimeoutMs)) return;
    throw new FreighterWalletError("NOT_INSTALLED", ACTIONABLE_MESSAGES.NOT_INSTALLED);
  }

  async getPublicKey(): Promise<string> {
    await this.ensureInjected();

    const access = await callFreighter(() => isAllowed());
    if (access.error) throw toFreighterWalletError(access.error);

    if (!access.isAllowed) {
      // Triggers Freighter's "connect this site" prompt.
      const permission = await callFreighter(() => setAllowed());
      if (permission.error) throw toFreighterWalletError(permission.error);
      if (!permission.isAllowed) {
        throw new FreighterWalletError(
          "REJECTED",
          'The Freighter connection request was not approved. Open the extension, click "Connect" to allow this site, and try again.',
        );
      }
    }

    const result = await callFreighter(() => getAddress());
    if (result.error) throw toFreighterWalletError(result.error);
    if (!result.address) {
      throw new FreighterWalletError(
        "UNKNOWN",
        "Freighter did not return a public key. Open the extension and make sure the wallet is unlocked.",
      );
    }
    return result.address;
  }

  async signTransaction(txXdr: string, networkPassphrase: string): Promise<string> {
    await this.ensureInjected();

    const networkResult = await callFreighter(() => getNetworkDetails());
    if (networkResult.error) throw toFreighterWalletError(networkResult.error);
    if (networkResult.networkPassphrase !== networkPassphrase) {
      throw new FreighterWalletError(
        "NETWORK_MISMATCH",
        `Freighter is connected to the wrong network (${networkResult.network || "unknown"}). Switch it to ${networkLabel(networkPassphrase)} in the Freighter extension and try again.`,
      );
    }

    const result = await callFreighter(() =>
      freighterSignTransaction(txXdr, { networkPassphrase }),
    );
    if (result.error) throw toFreighterWalletError(result.error);
    if (!result.signedTxXdr) {
      throw new FreighterWalletError(
        "REJECTED",
        "Freighter did not return a signed transaction. Approve the signing request in the extension — the popup may have been dismissed.",
      );
    }

    verifySignedTransactionMatchesOriginal(
      txXdr,
      result.signedTxXdr,
      networkPassphrase,
    );

    return result.signedTxXdr;
  }
}
