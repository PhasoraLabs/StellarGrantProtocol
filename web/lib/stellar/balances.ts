import { getHorizonClient } from "./client";
import type { Horizon } from "@stellar/stellar-sdk";

/** Raw balance entry as returned by the Horizon API */
type HorizonBalance = Horizon.HorizonApi.BalanceLine;

// ── Types ──────────────────────────────────────────────────────────────────

/** A single asset balance entry */
export interface GrantBalance {
  /** Asset code, e.g. "XLM" or "USDC" */
  assetCode: string;
  /** Asset issuer address. Empty string for native XLM. */
  assetIssuer: string;
  /** Whether this is the native XLM asset */
  isNative: boolean;
  /** Raw balance string as returned by the RPC (e.g. "100.0000000") */
  rawBalance: string;
  /** Balance as a BigInt in stroops (7 decimal places) */
  balanceStroops: bigint;
  /** Human-readable formatted amount (e.g. "100.0000000") */
  formatted: string;
}

/** Full balance snapshot for a grant contract account */
export interface GrantBalances {
  /** The contract account address whose balances are reported */
  contractAddress: string;
  /** All asset balances held by this account */
  balances: GrantBalance[];
  /** Ledger sequence number at time of fetch */
  ledger: number;
  /** UTC timestamp of the snapshot */
  fetchedAt: Date;
}

/** Options for balance change listeners */
export interface BalanceChangeListenerOptions {
  /** How often to poll for changes (ms). Default: 10_000 */
  pollInterval?: number;
  /** Callback invoked when balances change */
  onChange: (current: GrantBalances, previous: GrantBalances | null) => void;
  /** Callback invoked on fetch errors */
  onError?: (error: Error) => void;
}

// ── Balance parsing ────────────────────────────────────────────────────────

/**
 * Convert a decimal balance string (e.g. "123.4560000") to BigInt stroops.
 * Stellar uses 7 decimal places for all assets.
 */
export function parseBalanceToStroops(raw: string): bigint {
  const [whole = "0", frac = ""] = raw.split(".");
  const paddedFrac = frac.padEnd(7, "0").slice(0, 7);
  return BigInt(whole) * 10_000_000n + BigInt(paddedFrac);
}

/**
 * Format stroops back to a human-readable decimal string with 7 decimal places.
 */
export function formatStroops(stroops: bigint): string {
  const whole = stroops / 10_000_000n;
  const frac = (stroops % 10_000_000n).toString().padStart(7, "0");
  return `${whole}.${frac}`;
}

// ── Core fetch function ────────────────────────────────────────────────────

/**
 * Fetch the current XLM and token balances for a grant's contract account.
 *
 * @param contractAddress - The Stellar contract/account address of the grant
 * @returns A structured GrantBalances snapshot
 * @throws If the RPC call fails or the account does not exist
 */
export async function getGrantBalances(contractAddress: string): Promise<GrantBalances> {
  if (!contractAddress || contractAddress.trim() === "") {
    throw new Error("contractAddress must not be empty");
  }

  const horizon = getHorizonClient();

  // loadAccount() returns an AccountResponse with full balance/trustline data
  const account = await horizon.loadAccount(contractAddress);

  const balances: GrantBalance[] = (account.balances as HorizonBalance[]).map((b) => {
    const isNative = b.asset_type === "native";
    const raw = b.balance;
    const stroops = parseBalanceToStroops(raw);

    return {
      assetCode: isNative ? "XLM" : (b as Horizon.HorizonApi.BalanceLineAsset).asset_code,
      assetIssuer: isNative ? "" : (b as Horizon.HorizonApi.BalanceLineAsset).asset_issuer,
      isNative,
      rawBalance: raw,
      balanceStroops: stroops,
      formatted: formatStroops(stroops),
    };
  });

  // Put native XLM first, then sort remaining tokens alphabetically
  balances.sort((a, b) => {
    if (a.isNative) return -1;
    if (b.isNative) return 1;
    return a.assetCode.localeCompare(b.assetCode);
  });

  return {
    contractAddress,
    balances,
    ledger: Number(account.last_modified_ledger ?? 0),
    fetchedAt: new Date(),
  };
}

/**
 * Convenience helper: get the native XLM balance only.
 *
 * @param contractAddress - The grant contract address
 * @returns The XLM GrantBalance entry, or null if not found
 */
export async function getGrantXlmBalance(
  contractAddress: string
): Promise<GrantBalance | null> {
  const snapshot = await getGrantBalances(contractAddress);
  return snapshot.balances.find((b) => b.isNative) ?? null;
}

/**
 * Convenience helper: get the balance of a specific SAC token.
 *
 * @param contractAddress - The grant contract address
 * @param assetCode - The asset code to look for (e.g. "USDC")
 * @param assetIssuer - Optional issuer address for disambiguation
 * @returns The matching GrantBalance entry, or null if not found
 */
export async function getGrantTokenBalance(
  contractAddress: string,
  assetCode: string,
  assetIssuer?: string
): Promise<GrantBalance | null> {
  const snapshot = await getGrantBalances(contractAddress);
  return (
    snapshot.balances.find((b) => {
      const codeMatch = b.assetCode.toUpperCase() === assetCode.toUpperCase();
      const issuerMatch = assetIssuer ? b.assetIssuer === assetIssuer : true;
      return codeMatch && issuerMatch;
    }) ?? null
  );
}

// ── Balance change listener ────────────────────────────────────────────────

/**
 * Subscribe to balance changes for a grant contract account.
 *
 * Polls the RPC on a configurable interval and invokes `onChange` when
 * any balance differs from the previous snapshot.
 *
 * @param contractAddress - The grant contract address to monitor
 * @param options - Polling config and callbacks
 * @returns A cleanup function to stop listening
 */
export function listenToBalanceChanges(
  contractAddress: string,
  options: BalanceChangeListenerOptions
): () => void {
  const { pollInterval = 10_000, onChange, onError } = options;

  let previous: GrantBalances | null = null;
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | null = null;

  const poll = async () => {
    if (stopped) return;
    try {
      const current = await getGrantBalances(contractAddress);
      const hasChanged = hasBalanceChangedFrom(previous, current);
      if (hasChanged) {
        onChange(current, previous);
      }
      previous = current;
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      onError?.(error);
    }
    if (!stopped) {
      timer = setTimeout(poll, pollInterval);
    }
  };

  // Start immediately
  void poll();

  return () => {
    stopped = true;
    if (timer !== null) clearTimeout(timer);
  };
}

/**
 * Determine if any balance has changed between two snapshots.
 */
function hasBalanceChangedFrom(
  previous: GrantBalances | null,
  current: GrantBalances
): boolean {
  if (!previous) return true; // first fetch always triggers

  if (previous.balances.length !== current.balances.length) return true;

  for (const curr of current.balances) {
    const prev = previous.balances.find(
      (b) => b.assetCode === curr.assetCode && b.assetIssuer === curr.assetIssuer
    );
    if (!prev || prev.balanceStroops !== curr.balanceStroops) return true;
  }
  return false;
}
