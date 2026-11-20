import { rpc, scValToNative, xdr } from "@stellar/stellar-sdk";

/**
 * Represents a parsed contract event.
 * @template T The type of the event data.
 */
export interface ParsedEvent<T = any> {
  /** The name of the event (e.g., "GrantCreated"). */
  name: string;
  /** The decoded data associated with the event. */
  data: T;
  /** The ID of the contract that emitted the event. */
  contractId: string;
}

/**
 * Utility for parsing and filtering Soroban contract events.
 */
export class EventParser {
  /**
   * Extracts and decodes events from a successful transaction response.
   * 
   * This method handles both raw XDR objects and base64 strings returned by the RPC.
   * 
   * @param response The transaction response from the RPC server.
   * @returns An array of parsed events.
   */
  static parseEvents(response: rpc.Api.GetTransactionResponse): ParsedEvent[] {
    const successResponse = response as any;
    if (response.status !== "SUCCESS" || !successResponse.events) {
      return [];
    }

    return successResponse.events.map((event: any) => {
      const parseScVal = (val: any): xdr.ScVal => {
        if (typeof val === "string") {
          return xdr.ScVal.fromXDR(val, "base64");
        }
        return val as xdr.ScVal;
      };

      const topics = event.topic.map((t: any) => scValToNative(parseScVal(t)));
      const value = scValToNative(parseScVal(event.value));

      // For Soroban contract events:
      // Topic 0 is usually the event name symbol.
      const name = topics.length > 0 ? String(topics[0]) : "unknown";

      return {
        name,
        data: value,
        contractId: event.contractId,
      };
    });
  }

  /**
   * Helper to find the first event by name in an array of parsed events.
   * @param events Array of parsed events.
   * @param name The name of the event to look for.
   * @returns The first matching event or undefined.
   */
  static findEvent<T = any>(events: ParsedEvent[], name: string): ParsedEvent<T> | undefined {
    return events.find((e) => e.name === name);
  }

  /**
   * Helper to filter events by name in an array of parsed events.
   * @param events Array of parsed events.
   * @param name The name of the events to filter for.
   * @returns An array of matching events.
   */
  static filterEvents<T = any>(events: ParsedEvent[], name: string): ParsedEvent<T>[] {
    return events.filter((e) => e.name === name);
  }
}

/**
 * Data payload for the `GrantCreated` event.
 */
export interface GrantCreatedData {
  /** Schema version of the event. */
  event_version: number;
  /** The unique numeric ID of the new grant. */
  grant_id: bigint;
  /** The address of the grant owner. */
  owner: string;
  /** The project title. */
  title: string;
  /** The total budget allocated for the grant. */
  total_amount: bigint;
  /** List of tags associated with the grant. */
  tags: string[];
  /** Ledger timestamp when the grant was created. */
  timestamp: bigint;
}

/**
 * Data payload for the `MilestoneSubmitted` event.
 */
export interface MilestoneSubmittedData {
  /** Schema version of the event. */
  event_version: number;
  /** The unique numeric ID of the grant. */
  grant_id: bigint;
  /** The index of the milestone (0-based). */
  milestone_idx: number;
  /** Description or summary of the milestone work. */
  description: string;
  /** Ledger timestamp when the milestone was submitted. */
  timestamp: bigint;
}

/**
 * Data payload for the `GrantFunded` event.
 */
export interface GrantFundedData {
  /** Schema version of the event. */
  event_version: number;
  /** The unique numeric ID of the grant. */
  grant_id: bigint;
  /** The address of the funder. */
  funder: string;
  /** The amount of tokens added (base units). */
  amount: bigint;
  /** The address of the token used for funding. */
  token: string;
  /** The updated total balance of the grant. */
  new_balance: bigint;
  /** Ledger timestamp when the funding occurred. */
  timestamp: bigint;
}

/**
 * Data payload for the `MilestoneVoted` event.
 */
export interface MilestoneVotedData {
  /** Schema version of the event. */
  event_version: number;
  /** The unique numeric ID of the grant. */
  grant_id: bigint;
  /** The index of the milestone (0-based). */
  milestone_idx: number;
  /** The address of the reviewer who voted. */
  reviewer: string;
  /** Whether the milestone was approved (true) or rejected (false). */
  approve: boolean;
  /** Optional feedback provided by the reviewer. */
  feedback?: string;
  /** Ledger timestamp when the vote was cast. */
  timestamp: bigint;
}
