import {
  Contract,
  rpc,
  TransactionBuilder,
  nativeToScVal,
  scValToNative,
  xdr,
} from "@stellar/stellar-sdk";
import { parseSorobanError } from "./errors/parseSorobanError";
import { StellarGrantsError } from "./errors/StellarGrantsError";
import {
  GrantCreateInput,
  GrantFundInput,
  MilestoneSubmitInput,
  MilestoneVoteInput,
  StellarGrantsSDKConfig,
} from "./types";
import { meetsThreshold, PendingXdrStore } from "./utils/transactions";

export const CONTRACT_INTERFACE_VERSION = 1;

export class StellarGrantsSDK {
  public readonly pendingXdrStore = new PendingXdrStore();
  public readonly meetsThreshold = meetsThreshold;

  private readonly contract: Contract;
  private readonly server: any;
  private readonly config: StellarGrantsSDKConfig;
  private eventPollHandle: ReturnType<typeof setTimeout> | null = null;

  constructor(config: StellarGrantsSDKConfig) {
    if (!config.rpcUrl && !config.proxyUrl) {
      throw new Error("Either rpcUrl or proxyUrl must be provided.");
    }

    this.config = config;
    this.contract = new Contract(config.contractId);
    const serverUrl = config.proxyUrl ?? config.rpcUrl!;
    this.server = new rpc.Server(serverUrl, {
      allowHttp: serverUrl.startsWith("http://"),
      customHeaders: config.customHeaders,
    } as any);
  }

  /**
   * Creates a new grant.
   */
  async grantCreate(
    input: GrantCreateInput,
    options?: { feePriority?: "low" | "medium" | "high"; simulatedFee?: string },
  ): Promise<unknown> {
    return this.invokeWrite(
      "grant_create",
      [
        nativeToScVal(input.owner, { type: "address" }),
        nativeToScVal(input.title),
        nativeToScVal(input.description),
        nativeToScVal(input.budget, { type: "i128" }),
        nativeToScVal(input.deadline, { type: "u64" }),
        nativeToScVal(input.milestoneCount, { type: "u32" }),
      ],
      options,
    );
  }

  /**
   * Funds an existing grant.
   */
  async grantFund(input: GrantFundInput, options?: { feePriority?: "low" | "medium" | "high"; simulatedFee?: string }): Promise<unknown> {
    return this.invokeWrite(
      "grant_fund",
      [
        nativeToScVal(input.grantId, { type: "u32" }),
        nativeToScVal(input.token, { type: "address" }),
        nativeToScVal(input.amount, { type: "i128" }),
      ],
      options,
    );
  }

  /**
   * Submits milestone proof for a grant.
   */
  async milestoneSubmit(
    input: MilestoneSubmitInput,
    options?: { feePriority?: "low" | "medium" | "high"; simulatedFee?: string },
  ): Promise<unknown> {
    return this.invokeWrite(
      "milestone_submit",
      [
        nativeToScVal(input.grantId, { type: "u32" }),
        nativeToScVal(input.milestoneIdx, { type: "u32" }),
        nativeToScVal(input.proofHash),
      ],
      options,
    );
  }

  /**
   * Casts an approval/rejection vote for a milestone.
   */
  async milestoneVote(
    input: MilestoneVoteInput,
    options?: { feePriority?: "low" | "medium" | "high"; simulatedFee?: string },
  ): Promise<unknown> {
    return this.invokeWrite(
      "milestone_vote",
      [
        nativeToScVal(input.grantId, { type: "u32" }),
        nativeToScVal(input.milestoneIdx, { type: "u32" }),
        nativeToScVal(input.approve),
      ],
      options,
    );
  }

  /**
   * Reads a grant by id.
   */
  async grantGet(grantId: number): Promise<unknown> {
    return this.invokeRead("grant_get", [nativeToScVal(grantId, { type: "u32" })]);
  }

  /**
   * Reads milestone details by grant and milestone index.
   */
  async milestoneGet(grantId: number, milestoneIdx: number): Promise<unknown> {
    return this.invokeRead("milestone_get", [
      nativeToScVal(grantId, { type: "u32" }),
      nativeToScVal(milestoneIdx, { type: "u32" }),
    ]);
  }

  async getAllowance(token: string, owner: string): Promise<{ amount: bigint; expirationLedger: number }> {
    const raw = await this.invokeRead("allowance", [
      nativeToScVal(token, { type: "address" }),
      nativeToScVal(owner, { type: "address" }),
    ]);

    const payload = raw && typeof raw === "object" && "_native" in raw ? (raw as any)._native : raw;
    return {
      amount: BigInt((payload as any)?.amount ?? 0),
      expirationLedger: Number((payload as any)?.expiration_ledger ?? (payload as any)?.expirationLedger ?? 0),
    };
  }

  async checkAndSetAllowance(token: string, amount: bigint, owner: string): Promise<{ sufficient: boolean; current: bigint; required?: bigint }> {
    const current = (await this.getAllowance(token, owner))?.amount ?? BigInt(0);

    if (current >= amount) {
      return { sufficient: true, current };
    }

    await this.setAllowance(token, amount, owner);
    return { sufficient: false, current, required: amount };
  }

  async getAccountSigners(accountId: string): Promise<any> {
    return this.server.getAccount(accountId);
  }

  subscribeToEvents(callback: (event: any) => void): () => void {
    let active = true;

    const normalizeEvent = (raw: any) => ({
      id: raw.id,
      type: raw.type,
      contractId: raw.contractId ?? raw.contract_id,
      ledger: raw.ledger,
      timestamp: raw.timestamp,
      topic: raw.topic,
      value: raw.value ?? raw._value,
    });

    const poll = async () => {
      if (!active) return;
      try {
        const response = await this.server.getEvents();
        if (response?.events && Array.isArray(response.events)) {
          for (const rawEvent of response.events) {
            callback(normalizeEvent(rawEvent));
          }
        }
      } catch {
        // Swallow polling errors.
      }

      if (active) {
        this.eventPollHandle = setTimeout(poll, 0);
      }
    };

    poll();

    return () => {
      active = false;
      if (this.eventPollHandle) {
        clearTimeout(this.eventPollHandle);
        this.eventPollHandle = null;
      }
    };
  }

  async estimateFees(method: string, args: xdr.ScVal[], options?: { horizonUrl?: string; feePriority?: "low" | "medium" | "high"; simulatedFee?: string }): Promise<any> {
    const simulation = await this.server.simulateTransaction(await this.buildTx(method, args, options));
    if (simulation?.error) {
      throw new StellarGrantsError(String(simulation.error));
    }

    const minResourceFee = BigInt(simulation?.minResourceFee ?? "0");
    const base = options?.simulatedFee ? BigInt(options.simulatedFee) : minResourceFee;
    const feeStatsUrl = options?.horizonUrl ?? this.config.horizonUrl;

    if (feeStatsUrl) {
      try {
        const response = await fetch(`${feeStatsUrl.replace(/\/+$/, "")}/fee_stats`);
        if (response.ok) {
          const data = await response.json();
          const recommendedBase = BigInt(data?.max_fee?.p70 ?? Number(base));
          const usage = Number(data?.ledger_capacity_usage ?? 0);
          const networkLoad = usage > 0.85 ? "surge" : usage > 0.5 ? "moderate" : "normal";
          const low = (recommendedBase * BigInt(16)) / BigInt(10);
          const medium = (recommendedBase * BigInt(25)) / BigInt(10);
          const high = (recommendedBase * BigInt(35)) / BigInt(10);

          return {
            base: base.toString(),
            recommendedBase: recommendedBase.toString(),
            networkLoad,
            source: "horizon",
            low: low.toString(),
            medium: medium.toString(),
            high: high.toString(),
            modifiers: { low: 1.6, medium: 2.5, high: 3.5 },
          };
        }
      } catch {
        // Fall back to simulation fees.
      }
    }

    return {
      base: base.toString(),
      source: "simulation-fallback",
      low: base.toString(),
      medium: ((base * BigInt(15) + BigInt(9)) / BigInt(10)).toString(),
      high: (base * BigInt(2)).toString(),
      modifiers: { low: 1, medium: 1.5, high: 2 },
    };
  }

  async checkCompatibility(): Promise<{ compatible: boolean; sdkVersion: number; contractVersion: number | null; warning?: string }> {
    const sdkVersion = CONTRACT_INTERFACE_VERSION;

    try {
      const contractVersion = await this.invokeRead("sdk_version", []);
      if (typeof contractVersion !== "number") {
        return {
          compatible: true,
          sdkVersion,
          contractVersion: null,
          warning: "Could not determine contract interface version from response.",
        };
      }

      if (contractVersion === sdkVersion) {
        return { compatible: true, sdkVersion, contractVersion };
      }

      return {
        compatible: false,
        sdkVersion,
        contractVersion,
        warning: contractVersion > sdkVersion ? "The contract version is newer than the SDK; upgrade the SDK." : "The contract version is older than the SDK; consider upgrading the contract or using a compatible SDK version.",
      };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (message.toLowerCase().includes("method not found")) {
        return {
          compatible: true,
          sdkVersion,
          contractVersion: null,
          warning: "Could not determine contract interface version; falling back to compatibility mode.",
        };
      }
      throw error;
    }
  }

  private async setAllowance(token: string, amount: bigint, owner: string) {
    await this.invokeWrite("set_allowance", [
      nativeToScVal(token, { type: "address" }),
      nativeToScVal(amount, { type: "i128" }),
      nativeToScVal(owner, { type: "address" }),
    ]);
  }

  private async invokeRead(method: string, args: xdr.ScVal[]): Promise<unknown> {
    try {
      const tx = await this.buildTx(method, args);
      const simulation = await this.server.simulateTransaction(tx);
      this.ensureSimulationSuccess(simulation);
      return this.parseSimulationResult(simulation);
    } catch (error) {
      throw parseSorobanError(error);
    }
  }

  private async invokeWrite(method: string, args: xdr.ScVal[], options?: { feePriority?: "low" | "medium" | "high"; simulatedFee?: string }): Promise<unknown> {
    try {
      const tx = await this.buildTx(method, args, options);
      const simulation = await this.server.simulateTransaction(tx);
      this.ensureSimulationSuccess(simulation);

      const prepared = await this.server.prepareTransaction(tx);
      if (!this.config.signer) {
        throw new Error("A signer is required for write operations.");
      }

      const signedXdr = await this.config.signer.signTransaction(
        prepared.toXDR(),
        this.config.networkPassphrase,
      );
      const signedTx = TransactionBuilder.fromXDR(signedXdr, this.config.networkPassphrase);

      const sent = await this.server.sendTransaction(signedTx);
      if (sent.status === "ERROR") {
        throw new StellarGrantsError(`Send failed: ${sent.errorResult ?? "unknown error"}`);
      }
      return sent;
    } catch (error) {
      throw parseSorobanError(error);
    }
  }

  private async buildTx(method: string, args: xdr.ScVal[], options?: { feePriority?: "low" | "medium" | "high"; simulatedFee?: string }): Promise<any> {
    const signer = this.config.signer;
    if (!signer) {
      throw new Error("A signer is required to build a transaction.");
    }

    const source = await signer.getPublicKey();
    const account = await this.server.getAccount(source);
    const fee = options?.simulatedFee ?? this.config.defaultFee ?? "100";

    return new TransactionBuilder(account, {
      fee,
      networkPassphrase: this.config.networkPassphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(60)
      .build();
  }

  private ensureSimulationSuccess(simulation: any) {
    if (simulation?.error) {
      throw new StellarGrantsError(String(simulation.error));
    }
  }

  private parseSimulationResult(simulation: any): unknown {
    const retval = simulation?.result?.retval;
    if (!retval) return null;
    return scValToNative(retval);
  }
}
