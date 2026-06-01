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
import { TransactionTimeoutError } from "./errors/TransactionTimeoutError";
import { TransactionFailedError } from "./errors/TransactionFailedError";
import {
  GrantCreateInput,
  GrantFundInput,
  MilestoneSubmitInput,
  MilestoneVoteInput,
  StellarGrantsSDKConfig,
  TransactionResult,
  WaitForTransactionOptions,
  TransactionPollingStatus,
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

    // `wallet` takes precedence over `signer` when both are provided
    this.config = {
      ...config,
      signer: config.wallet ?? config.signer,
    };
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
    options?: { feePriority?: "low" | "medium" | "high"; simulatedFee?: string; footprint?: any },
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
  async grantFund(input: GrantFundInput, options?: { feePriority?: "low" | "medium" | "high"; simulatedFee?: string; footprint?: any }): Promise<unknown> {
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
   * Polls for transaction status until it reaches a terminal state.
   * Resolves with TransactionResult on SUCCESS, rejects on FAILED, timeout, or network error.
   */
  async waitForTransaction(
    hash: string,
    options?: WaitForTransactionOptions,
  ): Promise<TransactionResult> {
    const pollIntervalMs = options?.pollIntervalMs ?? 3000;
    const timeoutMs = options?.timeoutMs ?? 60000;
    const maxNetworkRetries = options?.maxNetworkRetries ?? 3;
    const signal = options?.signal;

    if (pollIntervalMs < 500) {
      throw new Error(`pollIntervalMs must be at least 500ms, got ${pollIntervalMs}ms`);
    }

    if (timeoutMs <= pollIntervalMs) {
      throw new Error(`timeoutMs (${timeoutMs}ms) must be greater than pollIntervalMs (${pollIntervalMs}ms)`);
    }

    if (signal?.aborted) {
      return Promise.reject(new StellarGrantsError("Transaction polling cancelled", "ABORTED"));
    }

    return new Promise((resolve, reject) => {
      let active = true;
      const startTime = Date.now();
      let attempt = 0;
      let consecutiveNetworkErrors = 0;
      let timeoutHandle: ReturnType<typeof setTimeout> | null = null;
      let abortListener: (() => void) | null = null;

      const cleanup = () => {
        active = false;
        if (timeoutHandle) clearTimeout(timeoutHandle);
        if (abortListener && signal) {
          signal.removeEventListener("abort", abortListener);
        }
      };

      const poll = async () => {
        if (!active) return;

        attempt++;
        const elapsedMs = Date.now() - startTime;

        try {
          const response = await this.server.getTransaction(hash);

          if (!active) return;

          consecutiveNetworkErrors = 0;
          const status = response?.status as TransactionPollingStatus;

          options?.onStatusChange?.(status);

          switch (status) {
            case "SUCCESS": {
              cleanup();
              resolve({
                status: "SUCCESS",
                ledger: response.ledger,
                envelopeXdr: response.envelopeXdr || response.envelope_xdr,
                resultXdr: response.resultXdr || response.result_xdr,
                resultMetaXdr: response.resultMetaXdr || response.result_meta_xdr,
                hash,
              });
              return;
            }
            case "FAILED": {
              cleanup();
              reject(new TransactionFailedError(hash, response?.errorResult, { raw: response }));
              return;
            }
            case "PENDING":
            case "DUPLICATE":
            case "TRY_AGAIN_LATER":
            case "NOT_FOUND":
              break;
            default:
              break;
          }

          options?.onPoll?.(attempt, elapsedMs);

          if (elapsedMs >= timeoutMs) {
            cleanup();
            reject(new TransactionTimeoutError(hash, timeoutMs));
            return;
          }

          if (!active) return;
          setTimeout(poll, pollIntervalMs);
        } catch (error) {
          if (!active) return;

          consecutiveNetworkErrors++;

          if (consecutiveNetworkErrors > maxNetworkRetries) {
            cleanup();
            reject(new StellarGrantsError(`Network error after ${maxNetworkRetries} retries: ${error}`, "NETWORK_ERROR", error));
            return;
          }

          options?.onPoll?.(attempt, Date.now() - startTime);

          if (!active) return;
          setTimeout(poll, pollIntervalMs);
        }
      };

      if (signal) {
        abortListener = () => {
          if (active) {
            cleanup();
            reject(new StellarGrantsError("Transaction polling cancelled", "ABORTED"));
          }
        };
        signal.addEventListener("abort", abortListener);
      }

      timeoutHandle = setTimeout(() => {
        if (active) {
          cleanup();
          reject(new TransactionTimeoutError(hash, timeoutMs));
        }
      }, timeoutMs);

      poll();
    });
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

  subscribeToEvents(
    callback: (event: any) => void,
    options?: {
      eventName?: string;
      startCursor?: string;
      pollIntervalMs?: number;
      maxRetries?: number;
      baseBackoffMs?: number;
      maxBackoffMs?: number;
      onError?: (error: any) => void;
      onStatusChange?: (status: "connecting" | "active" | "reconnecting" | "closed") => void;
    }
  ): () => void {
    let active = true;
    let cursor = options?.startCursor;
    let retryCount = 0;
    const maxRetries = options?.maxRetries ?? 10;
    const pollIntervalMs = options?.pollIntervalMs ?? 5000;
    const baseBackoffMs = options?.baseBackoffMs ?? 1000;
    const maxBackoffMs = options?.maxBackoffMs ?? 30000;
    const seenIds = new Set<string>();

    const normalizeEvent = (raw: any) => {
      let name = "unknown";
      try {
        if (raw.topic && Array.isArray(raw.topic) && raw.topic.length > 0) {
          const firstTopic = raw.topic[0];
          const scval = typeof firstTopic === "string" ? xdr.ScVal.fromXDR(firstTopic, "base64") : firstTopic;
          name = String(scValToNative(scval));
        }
      } catch {
        // ignore decoding errors
      }

      return {
        id: raw.id,
        type: raw.type,
        contractId: raw.contractId ?? raw.contract_id,
        ledger: raw.ledger,
        timestamp: raw.timestamp,
        topic: raw.topic,
        value: raw.value ?? raw._value,
        name,
        pagingToken: raw.pagingToken ?? raw.paging_token,
      };
    };

    const poll = async () => {
      if (!active) return;

      try {
        // Build getEvents filters and pagination
        const request: any = {
          filters: [
            {
              type: "contract",
              contractIds: [this.config.contractId],
            },
          ],
          pagination: {
            limit: 100,
          },
        };

        if (cursor) {
          request.pagination.cursor = cursor;
        }

        if (options?.eventName) {
          try {
            const nameScVal = nativeToScVal(options.eventName, { type: "symbol" });
            const nameXdr = nameScVal.toXDR("base64");
            request.filters[0].topics = [[nameXdr]];
          } catch {
            // fallback if encoding fails
          }
        }

        options?.onStatusChange?.("connecting");
        const response = await this.server.getEvents(request);

        if (!active) return;

        retryCount = 0;
        options?.onStatusChange?.("active");

        if (response?.events && Array.isArray(response.events)) {
          let latestCursor = cursor;
          const processedEvents: any[] = [];

          for (const rawEvent of response.events) {
            // Deduplicate
            if (rawEvent.id) {
              if (seenIds.has(rawEvent.id)) {
                continue;
              }
              seenIds.add(rawEvent.id);
            }

            const normalized = normalizeEvent(rawEvent);

            // Client-side event name filter backup
            if (options?.eventName && normalized.name !== options.eventName) {
              continue;
            }

            const token = rawEvent.pagingToken ?? rawEvent.paging_token;
            if (token && (!latestCursor || token > latestCursor)) {
              latestCursor = token;
            }

            processedEvents.push(normalized);
          }

          cursor = latestCursor;

          // Prune seenIds to prevent memory leak
          if (seenIds.size > 5000) {
            const arr = Array.from(seenIds);
            seenIds.clear();
            arr.slice(arr.length - 1000).forEach((id) => seenIds.add(id));
          }

          for (const ev of processedEvents) {
            callback(ev);
          }
        }

        if (active) {
          this.eventPollHandle = setTimeout(poll, pollIntervalMs);
        }
      } catch (err: any) {
        if (!active) return;

        retryCount++;
        options?.onStatusChange?.("reconnecting");

        if (retryCount > maxRetries) {
          active = false;
          options?.onStatusChange?.("closed");
          if (options?.onError) {
            options.onError(err);
          }
          return;
        }

        const delay = Math.random() * Math.min(maxBackoffMs, baseBackoffMs * 2 ** (retryCount - 1));
        this.eventPollHandle = setTimeout(poll, delay);
      }
    };

    poll();

    return () => {
      active = false;
      options?.onStatusChange?.("closed");
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

  private async invokeWrite(
    method: string,
    args: xdr.ScVal[],
    options?: {
      feePriority?: "low" | "medium" | "high";
      simulatedFee?: string;
      /** Pre-computed footprint from a prior {@link simulateFootprint} call. See issue #462. */
      footprint?: any;
      /** If true, wait for transaction confirmation before returning. */
      waitForConfirmation?: boolean;
      pollIntervalMs?: number;
      timeoutMs?: number;
    },
  ): Promise<unknown> {
    try {
      const tx = await this.buildTx(method, args, options);
      const simulation = await this.server.simulateTransaction(tx);
      this.ensureSimulationSuccess(simulation);

      // #458 — apply the simulation's minResourceFee when the caller has not
      // supplied an explicit fee so every write goes out with an accurate fee.
      const estimatedFee = simulation?.minResourceFee
        ? this.applyFeePriority(BigInt(simulation.minResourceFee), options?.feePriority)
        : undefined;

      const txForSending = estimatedFee
        ? await this.buildTx(method, args, { ...options, simulatedFee: estimatedFee.toString() })
        : tx;

      const prepared = await this.server.prepareTransaction(txForSending);
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

      if (options?.waitForConfirmation && sent.hash) {
        return this.waitForTransaction(sent.hash, {
          pollIntervalMs: options.pollIntervalMs,
          timeoutMs: options.timeoutMs,
        });
      }

      return sent;
    } catch (error) {
      throw parseSorobanError(error);
    }
  }

  /**
   * Simulate a transaction and return the ledger entry footprint for caching.
   *
   * Advanced callers can pass the returned footprint back via `options.footprint`
   * on subsequent write calls to skip redundant simulation round-trips for
   * repeated read-only access patterns. See [issue #462](https://github.com/StellarGrant/StellarGrant-fe/issues/462).
   *
   * @example
   * ```ts
   * const footprint = await sdk.simulateFootprint("grant_read", [grantIdVal]);
   * // Reuse across multiple calls without simulating each time:
   * await sdk.grantCreate(input, { footprint });
   * ```
   */
  async simulateFootprint(method: string, args: xdr.ScVal[]): Promise<any> {
    try {
      const tx = await this.buildTx(method, args);
      const simulation = await this.server.simulateTransaction(tx);
      this.ensureSimulationSuccess(simulation);
      return simulation?.transactionData ?? simulation?.footprint ?? null;
    } catch (error) {
      throw parseSorobanError(error);
    }
  }

  private applyFeePriority(
    base: bigint,
    priority?: "low" | "medium" | "high",
  ): bigint {
    switch (priority) {
      case "low":    return (base * BigInt(16)) / BigInt(10);
      case "high":   return (base * BigInt(35)) / BigInt(10);
      case "medium":
      default:       return (base * BigInt(25)) / BigInt(10);
    }
  }

  private async buildTx(
    method: string,
    args: xdr.ScVal[],
    options?: {
      feePriority?: "low" | "medium" | "high";
      simulatedFee?: string;
      footprint?: any;
    },
  ): Promise<any> {
    const signer = this.config.signer;
    if (!signer) {
      throw new Error("A signer is required to build a transaction.");
    }

    const source = await signer.getPublicKey();
    const account = await this.server.getAccount(source);
    const fee = options?.simulatedFee ?? this.config.defaultFee ?? "100";

    let builder = new TransactionBuilder(account, {
      fee,
      networkPassphrase: this.config.networkPassphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(60);

    // #462 — attach pre-computed footprint when provided
    if (options?.footprint) {
      builder = (builder as any).setSorobanData(options.footprint) ?? builder;
    }

    return builder.build();
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
