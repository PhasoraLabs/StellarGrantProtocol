/**
 * Native XLM Funding Support (Issue #504)
 *
 * Provides clean, transparent pathways for funding grants with native XLM.
 * Native XLM on Soroban is accessed via its Stellar Asset Contract (SAC),
 * which is deterministically derived from the network passphrase.
 *
 * Key concerns:
 *  - Detect whether a tokenAddress refers to native XLM ("native" sentinel or SAC ID).
 *  - Compute the SAC contract ID for native XLM on any network.
 *  - Build the funding XDR that routes through the XLM SAC (no manual wrapping required).
 *  - Compute the required balance, including the minimum Stellar reserve (1 XLM base).
 */

import {
  Asset,
  Contract,
  TransactionBuilder,
  BASE_FEE,
  nativeToScVal,
} from "@stellar/stellar-sdk";
import { horizonClient, rpcClient, networkPassphraseConfig } from "./client";
import { parseBalanceToStroops } from "./balances";
import type { ILogger } from "@/lib/logger";
import { Logger } from "@/lib/logger";

// ── Constants ──────────────────────────────────────────────────────────────

/** Minimum Stellar account reserve in stroops (1 XLM). */
export const STELLAR_BASE_RESERVE_STROOPS = 10_000_000n; // 1 XLM

/** Approximate transaction fee buffer in stroops (0.01 XLM). */
export const TX_FEE_BUFFER_STROOPS = 100_000n;

// ── Native XLM detection ───────────────────────────────────────────────────

/** Return the SAC contract ID for native XLM on the given network. */
export function getNativeXlmSacId(networkPassphrase?: string): string {
  return Asset.native().contractId(networkPassphrase ?? networkPassphraseConfig);
}

/**
 * Return true when `tokenAddress` is native XLM — either the "native"
 * sentinel string or the computed XLM SAC contract ID for the network.
 */
export function isNativeXlm(tokenAddress: string, networkPassphrase?: string): boolean {
  if (!tokenAddress) return false;
  if (tokenAddress.toLowerCase() === "native") return true;
  try {
    return tokenAddress === getNativeXlmSacId(networkPassphrase);
  } catch {
    return false;
  }
}

/**
 * Resolve a tokenAddress to its canonical SAC contract ID.
 * If `tokenAddress` is the "native" sentinel, returns the computed XLM SAC ID.
 * Otherwise returns the address unchanged (for SAC tokens like USDC).
 */
export function resolveTokenAddress(tokenAddress: string, networkPassphrase?: string): string {
  if (tokenAddress.toLowerCase() === "native") {
    return getNativeXlmSacId(networkPassphrase);
  }
  return tokenAddress;
}

// ── Balance helpers ────────────────────────────────────────────────────────

/** Shape returned by `computeRequiredBalance`. */
export interface XlmFundingRequirement {
  /** The raw amount the user wants to send (stroops). */
  fundingAmount: bigint;
  /** Transaction fee buffer (stroops). */
  feeBuffer: bigint;
  /** Minimum account reserve (stroops). */
  reserveBuffer: bigint;
  /** Total XLM the wallet must hold: fundingAmount + feeBuffer + reserveBuffer. */
  totalRequired: bigint;
  /** Human-readable total (e.g. "101.0100000"). */
  totalRequiredXlm: string;
}

/**
 * Compute the total XLM balance required to execute a native-XLM fund operation.
 * Accounts for the funding amount plus the minimum reserve and a fee buffer.
 */
export function computeRequiredBalance(fundingAmountStroops: bigint): XlmFundingRequirement {
  const totalRequired =
    fundingAmountStroops + TX_FEE_BUFFER_STROOPS + STELLAR_BASE_RESERVE_STROOPS;

  const whole = totalRequired / 10_000_000n;
  const frac = (totalRequired % 10_000_000n).toString().padStart(7, "0");

  return {
    fundingAmount: fundingAmountStroops,
    feeBuffer: TX_FEE_BUFFER_STROOPS,
    reserveBuffer: STELLAR_BASE_RESERVE_STROOPS,
    totalRequired,
    totalRequiredXlm: `${whole}.${frac}`,
  };
}

/**
 * Fetch the native XLM balance for a wallet address in stroops.
 * Returns 0n if the account does not exist or has no XLM.
 */
export async function getWalletXlmBalance(address: string): Promise<bigint> {
  try {
    const account = await horizonClient.loadAccount(address);
    const xlmEntry = account.balances.find((b) => b.asset_type === "native");
    if (!xlmEntry) return 0n;
    return parseBalanceToStroops(xlmEntry.balance);
  } catch {
    return 0n;
  }
}

/**
 * Check whether a wallet has sufficient XLM to fund a grant.
 * Returns `{ sufficient, balance, required }`.
 */
export async function checkXlmSufficiency(
  walletAddress: string,
  fundingAmountStroops: bigint
): Promise<{ sufficient: boolean; balance: bigint; required: XlmFundingRequirement }> {
  const balance = await getWalletXlmBalance(walletAddress);
  const required = computeRequiredBalance(fundingAmountStroops);
  return { sufficient: balance >= required.totalRequired, balance, required };
}

// ── XDR builder ───────────────────────────────────────────────────────────

export interface NativeXlmFundParams {
  /** Grant contract address (StellarGrants contract). */
  grantContractId: string;
  /** Numeric grant ID. */
  grantId: bigint;
  /** Amount in stroops (7 decimal places). */
  amountStroops: bigint;
  /** Wallet address of the funder. */
  funder: string;
  /** Optional network passphrase override. */
  networkPassphrase?: string;
  /** Optional logger. */
  logger?: ILogger;
}

/**
 * Build an unsigned XDR transaction for funding a grant with native XLM.
 *
 * Routing: the funder's wallet → XLM SAC `transfer` → StellarGrants `grant_fund`.
 * The XLM SAC handles the wrapping transparently; callers do not need to
 * manually approve or wrap their XLM balance.
 *
 * Returns the base64-encoded XDR envelope for the wallet layer to sign.
 */
export async function buildNativeXlmFundXdr(params: NativeXlmFundParams): Promise<string> {
  const log: ILogger = params.logger ?? new Logger({ prefix: "[xlm-native]" });

  const passphrase = params.networkPassphrase ?? networkPassphraseConfig;
  const xlmSacId = getNativeXlmSacId(passphrase);

  log.debug("buildNativeXlmFundXdr", {
    grantId: String(params.grantId),
    amount: String(params.amountStroops),
    xlmSacId,
    funder: params.funder,
  });

  const account = await horizonClient.loadAccount(params.funder);
  const ledger = await rpcClient.getLatestLedger();

  // The approve step mirrors USDC: allow the grant contract to pull the XLM.
  const expirationLedger = ledger.sequence + Math.ceil(86400 / 5);

  const tx = new TransactionBuilder(account, {
    fee: BASE_FEE,
    networkPassphrase: passphrase,
  })
    // Step 1: approve the grant contract to spend `amountStroops` of XLM via SAC
    .addOperation(
      new Contract(xlmSacId).call(
        "approve",
        nativeToScVal(params.funder, { type: "address" }),
        nativeToScVal(params.grantContractId, { type: "address" }),
        nativeToScVal(params.amountStroops, { type: "i128" }),
        nativeToScVal(expirationLedger, { type: "u32" })
      )
    )
    // Step 2: invoke grant_fund on the StellarGrants contract using the XLM SAC address
    .addOperation(
      new Contract(params.grantContractId).call(
        "grant_fund",
        nativeToScVal(params.funder, { type: "address" }),
        nativeToScVal(params.grantId, { type: "u64" }),
        nativeToScVal(xlmSacId, { type: "address" }), // token = native XLM SAC
        nativeToScVal(params.amountStroops, { type: "i128" })
      )
    )
    .setTimeout(30)
    .build();

  const xdrStr = tx.toEnvelope().toXDR("base64");
  log.debug("buildNativeXlmFundXdr XDR ready", { xdr: xdrStr });
  return xdrStr;
}
