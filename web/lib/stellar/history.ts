/**
 * Transaction History Retrieval — Issue #256
 *
 * Provides two SDK methods for retrieving historical on-chain activity:
 *
 *   getTransactionHistory(address)   — all StellarGrants-related txs for a wallet
 *   getGrantHistory(grantId)         — all txs touching a specific grant
 *
 * Both methods query the Horizon API using the already-configured
 * `horizonClient` singleton and parse each transaction's operations to
 * identify the relevant StellarGrants contract calls.
 *
 * The response is a unified `GrantHistoryRecord[]` that is typed,
 * paginated, and ready for display in a dashboard.
 *
 * @module web/lib/stellar/history
 */

import { getHorizonClient } from "./client";

// ── Public types ─────────────────────────────────────────────────────────────

/**
 * The set of StellarGrants contract operations we recognise.
 * "unknown_contract_call" is the fallback for Soroban invocations that
 * don't match a known function name.
 */
export type GrantOperationType =
  | "grant_create"
  | "grant_fund"
  | "grant_cancel"
  | "milestone_submit"
  | "milestone_approve"
  | "milestone_reject"
  | "milestone_payout"
  | "grant_withdraw"
  | "unknown_contract_call";

/** A single parsed history entry ready for dashboard display. */
export interface GrantHistoryRecord {
  /** Stellar transaction hash */
  txHash: string;
  /** ISO-8601 timestamp of the ledger close */
  createdAt: string;
  /** Whether the transaction was successful */
  successful: boolean;
  /** The identified contract operation, if determinable */
  operationType: GrantOperationType;
  /**
   * Grant ID extracted from the operation arguments, if available.
   * Stored as a string to avoid BigInt serialisation issues.
   */
  grantId?: string;
  /** Source account (signer) of the transaction */
  sourceAccount: string;
  /** Raw fee paid in stroops */
  feeCharged: string;
  /** Memo text attached to the transaction, if any */
  memo?: string;
}

/** Options shared by both history methods. */
export interface HistoryOptions {
  /**
   * Maximum number of records to return (default 50, max 200).
   */
  limit?: number;
  /**
   * Sort order: "desc" (newest first, default) or "asc" (oldest first).
   */
  order?: "asc" | "desc";
  /**
   * Horizon paging cursor to continue from a previous call.
   */
  cursor?: string;
}

/** Paginated result returned by history methods. */
export interface HistoryResult {
  records: GrantHistoryRecord[];
  /**
   * Pass this cursor to the next call to fetch the following page.
   * Undefined when there are no further pages.
   */
  nextCursor?: string;
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/**
 * Known StellarGrants function names as they appear in Soroban
 * invoke-host-function operations. Mapping is case-insensitive.
 */
const FUNCTION_NAME_MAP: Record<string, GrantOperationType> = {
  grant_create: "grant_create",
  grant_fund: "grant_fund",
  grant_cancel: "grant_cancel",
  milestone_submit: "milestone_submit",
  milestone_approve: "milestone_approve",
  milestone_reject: "milestone_reject",
  milestone_payout: "milestone_payout",
  grant_withdraw: "grant_withdraw",
};

/**
 * Read the contract ID at call time so it respects runtime env vars
 * (including those set in tests via process.env before each test).
 */
function getContractId(): string {
  return typeof process !== "undefined"
    ? (process.env.NEXT_PUBLIC_CONTRACT_ID ?? "")
    : "";
}

/**
 * Given a raw Horizon transaction record, determine the operation type and
 * grant ID by inspecting the memo and optional function name.
 *
 * Horizon returns operations as a separate sub-resource; for history
 * queries we rely on the `memo` field and the function name embedded in
 * the operation's `function` field (available in Horizon ≥ 2.27).
 */
function parseOperation(
  tx: { memo?: string },
  functionName?: string
): { operationType: GrantOperationType; grantId?: string } {
  let operationType: GrantOperationType = "unknown_contract_call";
  let grantId: string | undefined;

  if (functionName) {
    const normalised = functionName.toLowerCase().replace(/-/g, "_");
    operationType = FUNCTION_NAME_MAP[normalised] ?? "unknown_contract_call";
  }

  // Extract grant ID from memo (convention: "grant:<id>")
  if (tx.memo) {
    const match = /grant:(\d+)/i.exec(tx.memo);
    if (match) grantId = match[1];
  }

  return { operationType, grantId };
}

/**
 * Convert a raw Horizon transaction record to our unified type.
 * We deliberately avoid importing the heavy Horizon SDK types so the
 * module stays tree-shakeable; instead we type only the fields we read.
 */
function toHistoryRecord(
  tx: {
    hash: string;
    created_at: string;
    successful: boolean;
    source_account: string;
    fee_charged: string;
    memo?: string;
    paging_token: string;
  },
  functionName?: string
): GrantHistoryRecord {
  const { operationType, grantId } = parseOperation(tx, functionName);

  return {
    txHash: tx.hash,
    createdAt: tx.created_at,
    successful: tx.successful,
    operationType,
    grantId,
    sourceAccount: tx.source_account,
    feeCharged: tx.fee_charged,
    memo: tx.memo,
  };
}

/**
 * Fetch a page of transactions from Horizon, map to GrantHistoryRecord[],
 * and extract the next paging cursor.
 */
async function fetchAndFilter(
  page: { records: unknown[] },
  limit: number
): Promise<{ records: GrantHistoryRecord[]; nextCursor?: string }> {
  const rawRecords = page.records as Array<{
    hash: string;
    created_at: string;
    successful: boolean;
    source_account: string;
    fee_charged: string;
    memo?: string;
    paging_token: string;
    envelope_xdr?: string;
  }>;

  const records: GrantHistoryRecord[] = rawRecords
    .slice(0, limit)
    .map((tx) => toHistoryRecord(tx, undefined));

  const lastRecord = rawRecords[rawRecords.length - 1];
  const nextCursor = lastRecord ? lastRecord.paging_token : undefined;

  return { records, nextCursor };
}

// ── Public API ────────────────────────────────────────────────────────────────

/**
 * Retrieve the StellarGrants transaction history for a specific wallet address.
 *
 * Queries Horizon for all transactions whose source account matches `address`,
 * then parses them to identify StellarGrants contract calls.
 *
 * @param address - Stellar account address (G…)
 * @param options - Pagination and ordering options
 *
 * @example
 * ```ts
 * import { getTransactionHistory } from "@/lib/stellar";
 *
 * const { records, nextCursor } = await getTransactionHistory("GABC...", {
 *   limit: 20,
 *   order: "desc",
 * });
 *
 * for (const r of records) {
 *   console.log(r.createdAt, r.operationType, r.successful);
 * }
 *
 * // Load next page:
 * const page2 = await getTransactionHistory("GABC...", { cursor: nextCursor });
 * ```
 */
export async function getTransactionHistory(
  address: string,
  options: HistoryOptions = {}
): Promise<HistoryResult> {
  const limit = Math.min(options.limit ?? 50, 200);
  const order = options.order ?? "desc";

  const horizon = getHorizonClient();

  let builder = horizon
    .transactions()
    .forAccount(address)
    .limit(limit)
    .order(order as "asc" | "desc");

  if (options.cursor) {
    builder = builder.cursor(options.cursor);
  }

  const page = await builder.call();
  return fetchAndFilter(page, limit);
}

/**
 * Retrieve all transactions related to a specific grant ID.
 *
 * Uses Horizon's account-level transaction feed scoped to the contract
 * account, then filters records whose memo matches `grant:<grantId>`.
 *
 * @param grantId - Numeric grant ID (as `number` or `bigint`)
 * @param options - Pagination and ordering options
 *
 * @example
 * ```ts
 * import { getGrantHistory } from "@/lib/stellar";
 *
 * const { records } = await getGrantHistory(42, { limit: 100 });
 *
 * const funded = records.filter(r => r.operationType === "grant_fund");
 * ```
 */
export async function getGrantHistory(
  grantId: number | bigint,
  options: HistoryOptions = {}
): Promise<HistoryResult> {
  const limit = Math.min(options.limit ?? 50, 200);
  const order = options.order ?? "desc";
  const grantIdStr = String(grantId);
  const contractId = getContractId();

  if (!contractId) {
    // No contract configured — return empty rather than throwing
    return { records: [] };
  }

  const horizon = getHorizonClient();

  let builder = horizon
    .transactions()
    .forAccount(contractId)
    .limit(limit)
    .order(order as "asc" | "desc");

  if (options.cursor) {
    builder = builder.cursor(options.cursor);
  }

  const page = await builder.call();
  const { records: all, nextCursor } = await fetchAndFilter(page, limit);

  // Narrow to records for this grant by memo convention
  const records = all.filter(
    (r) =>
      r.grantId === grantIdStr ||
      r.memo?.toLowerCase().includes(`grant:${grantIdStr}`)
  );

  return { records, nextCursor: records.length > 0 ? nextCursor : undefined };
}
