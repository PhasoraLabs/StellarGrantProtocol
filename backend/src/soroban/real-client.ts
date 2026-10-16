/**
 * RealSorobanClient
 *
 * Production implementation of SorobanContractClient that communicates with
 * the live Stellar network via the Soroban RPC server. The MockSorobanContractClient
 * remains available for local development and tests; select via USE_MOCK_SOROBAN env.
 */

import { rpc, xdr, scValToNative } from "@stellar/stellar-sdk";
import { env } from "../config/env";
import type {
  SorobanContractClient,
  SorobanGrant,
  SorobanContractEvent,
  SorobanMilestone,
} from "./types";
import { decodeU64, decodeString, decodeAddress, decodeVec, decodeMap, decodeI128AsString } from "./decode";

export class RealSorobanClient implements SorobanContractClient {
  private static readonly FETCH_RETRIES = 3;
  private static readonly FETCH_BACKOFF_MS = 100;
  private readonly server: rpc.Server;
  private readonly contractId: string;

  constructor() {
    this.server = new rpc.Server(env.rpcUrl);
    this.contractId = env.contractId;
  }

  // ── Public interface ──────────────────────────────────────────────────

  async fetchGrants() {
    const count = await this.callView<number>("grant_count", []);
    const grants: SorobanGrant[] = [];
    const failedIds: number[] = [];
    for (let i = 1; i <= count; i++) {
      try {
        const grant = await this.fetchGrantWithRetry(i);
        if (grant) grants.push(grant);
      } catch (error) {
        failedIds.push(i);
        console.warn(`Failed to fetch grant ${i} after retries`, error);
      }
    }
    return { grants, failedIds };
  }

  private async fetchGrantWithRetry(id: number): Promise<SorobanGrant | null> {
    let lastError: unknown;
    for (let attempt = 0; attempt < RealSorobanClient.FETCH_RETRIES; attempt++) {
      try {
        return await this.fetchGrantById(id);
      } catch (error) {
        lastError = error;
        if (attempt < RealSorobanClient.FETCH_RETRIES - 1) {
          await new Promise((resolve) =>
            setTimeout(resolve, RealSorobanClient.FETCH_BACKOFF_MS * 2 ** attempt),
          );
        }
      }
    }
    throw lastError instanceof Error
      ? lastError
      : new Error(`Failed to fetch grant ${id}`);
  }

  async fetchGrantById(id: number): Promise<SorobanGrant | null> {
    const arg = xdr.ScVal.scvU64(xdr.Uint64.fromNumber(id));
    const result = await this.callView<xdr.ScVal | null>("grant_get", [arg]);
    if (!result) return null;
    return this.decodeGrant(result);
  }

  async getLatestLedger(): Promise<number> {
    const info = await this.server.getLatestLedger();
    return info.sequence;
  }

  async fetchEvents(
    fromLedger: number,
    toLedger: number,
  ): Promise<SorobanContractEvent[]> {
    const allEvents: SorobanContractEvent[] = [];
    let cursor: string | undefined;

    // Paginate through all events in the ledger range (#1245)
    do {
      const response = await this.server.getEvents({
        startLedger: fromLedger,
        endLedger: toLedger,
        limit: 200,
        filters: [
          {
            type: "contract",
            contractIds: [this.contractId],
          },
        ],
      });

      allEvents.push(...response.events.map((e) => this.decodeEvent(e)));
      cursor = response.cursor;
    } while (cursor);

    return allEvents.filter((e) => e.ledger <= toLedger);
  }

  // ── Private helpers ───────────────────────────────────────────────────

  private async callView<T>(
    method: string,
    args: xdr.ScVal[],
  ): Promise<T> {
    const result = await this.server.simulateTransaction(
      await this.buildViewTx(method, args),
    );

    if (rpc.Api.isSimulationError(result)) {
      throw new Error(`Soroban view call "${method}" failed: ${result.error}`);
    }

    if (!result.result) {
      throw new Error(`Soroban view call "${method}" returned no simulation result`);
    }

    const retval = result.result.retval;
    const native = scValToNative(retval);

    // For grant_count the contract returns a u64 → convert to number
    if (typeof native === "bigint") return Number(native) as unknown as T;
    // For grant_get the contract returns a map or void
    if (retval.switch() === xdr.ScValType.scvVoid()) return null as unknown as T;
    return retval as unknown as T;
  }

  private async buildViewTx(method: string, args: xdr.ScVal[]) {
    // We need a dummy source account for simulation; use a well-known address.
    const { TransactionBuilder, Account, Contract, Networks } = await import(
      "@stellar/stellar-sdk"
    );

    const dummySource = new Account(
      "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN",
      "0",
    );

    const contract = new Contract(this.contractId);

    return new TransactionBuilder(dummySource, {
      fee: "100",
      networkPassphrase: env.networkPassphrase,
    })
      .addOperation(contract.call(method, ...args))
      .setTimeout(30)
      .build();
  }

  private decodeGrant(val: xdr.ScVal): SorobanGrant {
    const m = decodeMap(val);

    const id = m.has("id") ? decodeU64(m.get("id")!) : 0;
    const title = m.has("title") ? decodeString(m.get("title")!) : "";
    const status = m.has("status") ? decodeString(m.get("status")!) : "unknown";
    const recipient = m.has("recipient")
      ? decodeAddress(m.get("recipient")!)
      : "";
    const totalAmount = m.has("total_amount")
      ? decodeI128AsString(m.get("total_amount")!)
      : "0";
    const owner = m.has("owner")
      ? decodeAddress(m.get("owner")!)
      : undefined;
    const token = m.has("token")
      ? decodeAddress(m.get("token")!)
      : "native";

    let milestones: SorobanMilestone[] | undefined;
    if (m.has("milestones")) {
      const vec = decodeVec(m.get("milestones")!);
      milestones = vec.map((item) => this.decodeMilestone(item));
    }

    return { id, title, status, recipient, totalAmount, owner, token, milestones };
  }

  private decodeMilestone(val: xdr.ScVal): SorobanMilestone {
    const m = decodeMap(val);
    return {
      idx: m.has("idx") ? decodeU64(m.get("idx")!) : 0,
      title: m.has("title") ? decodeString(m.get("title")!) : "",
      description: m.has("description")
        ? decodeString(m.get("description")!)
        : undefined,
      deadline: m.has("deadline") ? decodeString(m.get("deadline")!) : "",
    };
  }

  private decodeEvent(raw: rpc.Api.EventResponse): SorobanContractEvent {
    const typeTag =
      raw.topic.length > 0
        ? String(scValToNative(raw.topic[0]))
        : "unknown";

    const grantIdRaw =
      raw.topic.length > 1 ? scValToNative(raw.topic[1]) : null;
    const grantId =
      typeof grantIdRaw === "bigint"
        ? Number(grantIdRaw)
        : typeof grantIdRaw === "number"
        ? grantIdRaw
        : 0;

    const actorRaw =
      raw.topic.length > 2 ? String(scValToNative(raw.topic[2])) : "";

    const data = scValToNative(raw.value) as Record<string, unknown>;

    return {
      id: raw.id,
      grantId,
      type: typeTag,
      actorAddress: actorRaw,
      ledger: raw.ledger,
      data: typeof data === "object" && data !== null ? data : { raw: data },
    };
  }
}
