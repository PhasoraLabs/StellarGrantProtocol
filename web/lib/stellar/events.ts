/**
 * Event Streaming Utilities
 * 
 * Helpers for fetching and decoding StellarGrants contract events.
 */

import { rpc, scValToNative, xdr } from "@stellar/stellar-sdk";
import { getRpcClient } from "./client";
import { CONTRACT_ID } from "@/lib/constants";

export interface ContractEvent {
  type: string;
  data: Record<string, unknown>;
  ledger: number;
  timestamp: Date;
}

export interface FetchContractEventsOptions {
  /** First ledger to scan. Defaults to `latest - DEFAULT_LOOKBACK_LEDGERS`. */
  startLedger?: number;
  /** Contract to read events from. Defaults to the configured contract. */
  contractId?: string;
}

/** ~1 hour of ledgers at 5s per ledger; keeps the first poll cheap. */
export const DEFAULT_LOOKBACK_LEDGERS = 720;
const PAGE_LIMIT = 200;
const MAX_PAGES = 25;

/** Make decoded values JSON-safe (bigint → string, Buffer → hex). */
function toJsonSafe(value: unknown): unknown {
  if (typeof value === "bigint") return value.toString();
  if (value instanceof Uint8Array) return Buffer.from(value).toString("hex");
  if (Array.isArray(value)) return value.map(toJsonSafe);
  if (value instanceof Map) {
    return Object.fromEntries([...value].map(([k, v]) => [String(k), toJsonSafe(v)]));
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([k, v]) => [k, toJsonSafe(v)]),
    );
  }
  return value;
}

/** `grant_funded` → `GrantFunded`; already-Pascal names pass through. */
function toEventName(topic: string): string {
  return topic
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

/** True when the event payload references `grantId` (as `grant_id`). */
function matchesGrant(event: ContractEvent, grantId: string): boolean {
  if (!grantId) return true;
  const id = event.data.grant_id;
  return id !== undefined && String(id) === grantId;
}

/**
 * Fetch contract events for a specific grant via Soroban RPC `getEvents`,
 * following the cursor until every page in the range has been read.
 */
export async function fetchContractEvents(
  grantId: string,
  options: FetchContractEventsOptions = {},
): Promise<ContractEvent[]> {
  const contractId = options.contractId ?? CONTRACT_ID;
  if (!contractId) return [];

  const server = getRpcClient();
  const filters = [{ type: "contract" as const, contractIds: [contractId] }];

  let startLedger = options.startLedger;
  if (startLedger === undefined) {
    const latest = await server.getLatestLedger();
    startLedger = Math.max(1, latest.sequence - DEFAULT_LOOKBACK_LEDGERS);
  }

  const events: ContractEvent[] = [];
  let cursor: string | undefined;

  for (let page = 0; page < MAX_PAGES; page++) {
    // RPC rejects startLedger together with a cursor, so only the first page sends it.
    const response: rpc.Api.GetEventsResponse = await server.getEvents(
      cursor
        ? { filters, cursor, limit: PAGE_LIMIT }
        : { filters, startLedger, limit: PAGE_LIMIT },
    );

    for (const raw of response.events) {
      const decoded = decodeRpcEvent(raw);
      if (decoded && matchesGrant(decoded, grantId)) events.push(decoded);
    }

    if (response.events.length < PAGE_LIMIT || !response.cursor || response.cursor === cursor) {
      break;
    }
    cursor = response.cursor;
  }

  return events;
}

function decodeRpcEvent(raw: rpc.Api.EventResponse): ContractEvent | null {
  if (!raw.topic?.length) return null;
  try {
    const name = scValToNative(raw.topic[0]);
    const native = toJsonSafe(scValToNative(raw.value));
    const data =
      native && typeof native === "object" && !Array.isArray(native)
        ? (native as Record<string, unknown>)
        : { value: native };
    return {
      type: toEventName(String(name)),
      data,
      ledger: raw.ledger,
      timestamp: new Date(raw.ledgerClosedAt),
    };
  } catch {
    return null;
  }
}

/**
 * Decode event data from XDR format
 */
export function decodeEvent(eventXdr: string): ContractEvent | null {
  try {
    const event = xdr.ContractEvent.fromXDR(eventXdr, "base64");
    const body = event.body().v0();
    const topics = body.topics();
    if (!topics.length) return null;
    const native = toJsonSafe(scValToNative(body.data()));
    return {
      type: toEventName(String(scValToNative(topics[0]))),
      data:
        native && typeof native === "object" && !Array.isArray(native)
          ? (native as Record<string, unknown>)
          : { value: native },
      ledger: 0,
      timestamp: new Date(),
    };
  } catch {
    return null;
  }
}
