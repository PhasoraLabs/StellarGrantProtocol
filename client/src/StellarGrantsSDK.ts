import {
  Contract,
  Horizon,
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
  IpfsUploadConfig,
  GrantBalance,
  GrantBalances,
  BalanceChangeListenerOptions,
  GrantOperationType,
  GrantHistoryRecord,
  HistoryOptions,
  HistoryResult,
} from "./types";
import { meetsThreshold, PendingXdrStore } from "./utils/transactions";
import { combineSignatures } from "./utils/transactions";
import { uploadMetadataToIPFS, fetchMetadataFromIPFS } from "./ipfs";
import { BatchBuilder, BatchCall, BatchOperationError, BatchSendOptions } from "./batch/BatchBuilder";

export const CONTRACT_INTERFACE_VERSION = 1;

type FeePriority = "low" | "medium" | "high";

type WriteOptions = {
  /** Fee priority strategy. Ignored when `fee` or `simulatedFee` are provided. */
  feePriority?: FeePriority;
  /** Explicit fee to use (stroops). Highest precedence. */
  fee?: string;
  /** Explicit pre-computed min resource fee to use (stroops). */
  simulatedFee?: string;
  /** Pre-computed footprint / transactionData. */
  footprint?: any;
  /** If true, return prepared unsigned transaction XDR instead of submitting. */
  returnUnsignedXdr?: boolean;
  /** If true, wait for transaction confirmation before returning (only on submit). */
  waitForConfirmation?: boolean;
  pollIntervalMs?: number;
  timeoutMs?: number;
};

export class StellarGrantsSDK {
  public readonly pendingXdrStore = new PendingXdrStore();
  public readonly meetsThreshold = meetsThreshold;

  private readonly contract: Contract;
  private readonly server: any;
  private readonly config: StellarGrantsSDKConfig;
  private _horizonServer: Horizon.Server | null = null;
  private eventPollHandle: ReturnType<typeof setTimeout> | null = null;
  private eventHeartbeatHandle: ReturnType<typeof setTimeout> | null = null;
  private eventWs: WebSocket | null = null;

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
    if (config.horizonUrl) {
      this._horizonServer = new Horizon.Server(config.horizonUrl);
    }
  }

  private get horizonServer(): Horizon.Server {
    if (!this._horizonServer) {
      throw new StellarGrantsError(
        "horizonUrl is required for this operation. Pass it in the SDK config.",
        "NETWORK_ERROR",
      );
    }
    return this._horizonServer;
  }

  /**
   * Creates a new grant.
   * 
   * @param input Grant creation parameters
   * @param options Transaction options including IPFS configuration
   * @returns Transaction result
   * 
   * @example
   * ```typescript
   * // With IPFS metadata upload
   * const result = await sdk.grantCreate(
   *   {
   *     owner: 'G...',
   *     title: 'My Grant',
   *     description: 'ipfs://Qm...', // Will be auto-uploaded if ipfsConfig provided
   *     budget: BigInt(1000000),
   *     deadline: BigInt(Date.now() / 1000 + 86400 * 30),
   *     milestoneCount: 3
   *   },
   *   {
   *     ipfsConfig: { pinataJwt: process.env.PINATA_JWT },
   *     uploadMetadata: true
   *   }
   * );
   * ```
   */
  async grantCreate(
    input: GrantCreateInput,
    options?: { 
      feePriority?: "low" | "medium" | "high"; 
      fee?: string;
      simulatedFee?: string; 
      footprint?: any;
      ipfsConfig?: IpfsUploadConfig;
      uploadMetadata?: boolean;
    },
  ): Promise<unknown> {
    let description = input.description;

    // Auto-upload metadata to IPFS if requested
    if (options?.uploadMetadata && options?.ipfsConfig) {
      const metadata = {
        title: input.title,
        description: input.description,
        budget: input.budget.toString(),
        deadline: input.deadline.toString(),
        milestoneCount: input.milestoneCount,
        owner: input.owner,
      };

      const { cid } = await uploadMetadataToIPFS(metadata, options.ipfsConfig);
      description = `ipfs://${cid}`;
    }

    return this.invokeWrite(
      "grant_create",
      [
        nativeToScVal(input.owner, { type: "address" }),
        nativeToScVal(input.title),
        nativeToScVal(description),
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
    options?: { feePriority?: "low" | "medium" | "high"; fee?: string; simulatedFee?: string },
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
    options?: { feePriority?: "low" | "medium" | "high"; fee?: string; simulatedFee?: string },
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
   * 
   * @param grantId Grant identifier
   * @param options Options including IPFS gateway fallbacks
   * @returns Grant data with metadata fetched from IPFS if applicable
   * 
   * @example
   * ```typescript
   * const grant = await sdk.grantGet(1, {
   *   fetchIpfsMetadata: true,
   *   ipfsGateways: ['https://gateway.pinata.cloud/ipfs/']
   * });
   * ```
   */
  async grantGet(
    grantId: number,
    options?: {
      fetchIpfsMetadata?: boolean;
      ipfsGateways?: string[];
    }
  ): Promise<unknown> {
    const grant = await this.invokeRead("grant_get", [nativeToScVal(grantId, { type: "u32" })]);

    // Fetch IPFS metadata if description is an IPFS CID
    if (options?.fetchIpfsMetadata && grant && typeof grant === 'object') {
      const description = (grant as any).description || '';
      if (description.startsWith('ipfs://')) {
        const cid = description.replace('ipfs://', '');
        try {
          const metadata = await fetchMetadataFromIPFS(cid, options.ipfsGateways);
          return { ...grant, metadata, description };
        } catch (error) {
          // Log but don't fail if IPFS fetch fails
          console.warn(`Failed to fetch IPFS metadata for CID ${cid}:`, error);
        }
      }
    }

    return grant;
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
      heartbeatTimeoutMs?: number;
      /**
       * Optional WebSocket URL for providers that support streaming.
       * When omitted, the SDK uses HTTP polling only.
       */
      websocketUrl?: string;
      /**
       * Optional persistent cursor storage (e.g. localStorage).
       * If provided, cursor is loaded on start and saved after each successful page.
       */
      cursorStore?: { get: () => string | null; set: (cursor: string) => void };
      onError?: (error: any) => void;
      onStatusChange?: (status: "connecting" | "active" | "reconnecting" | "closed") => void;
    }
  ): () => void {
    let active = true;
    let cursor = options?.cursorStore?.get?.() ?? options?.startCursor;
    let retryCount = 0;
    const maxRetries = options?.maxRetries ?? 10;
    const pollIntervalMs = options?.pollIntervalMs ?? 5000;
    const baseBackoffMs = options?.baseBackoffMs ?? 1000;
    const maxBackoffMs = options?.maxBackoffMs ?? 30000;
    const heartbeatTimeoutMs = options?.heartbeatTimeoutMs ?? 60000;
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

    const cleanup = () => {
      if (this.eventPollHandle) {
        clearTimeout(this.eventPollHandle);
        this.eventPollHandle = null;
      }
      if (this.eventHeartbeatHandle) {
        clearTimeout(this.eventHeartbeatHandle);
        this.eventHeartbeatHandle = null;
      }
      if (this.eventWs) {
        try {
          this.eventWs.onclose = null;
          this.eventWs.onerror = null;
          this.eventWs.onmessage = null;
          this.eventWs.close();
        } catch {
          // ignore
        }
        this.eventWs = null;
      }
    };

    const resetHeartbeat = () => {
      if (this.eventHeartbeatHandle) clearTimeout(this.eventHeartbeatHandle);
      this.eventHeartbeatHandle = setTimeout(() => {
        if (!active) return;
        // No successful poll within window → force reconnect.
        cleanup();
        scheduleReconnect();
      }, heartbeatTimeoutMs);
    };

    const tryWebSocket = (): boolean => {
      // Only attempt WS in environments that support it.
      if (typeof WebSocket === "undefined") return false;

      // Only use WebSocket when the caller explicitly provides a ws(s) URL.
      // Most Soroban RPC providers expose HTTP JSON-RPC only.
      const wsUrl = options?.websocketUrl?.replace(/\/+$/, "");
      if (!wsUrl || !/^wss?:\/\//i.test(wsUrl)) return false;

      let ws: WebSocket;
      try {
        ws = new WebSocket(wsUrl);
      } catch {
        return false;
      }

      this.eventWs = ws;

      ws.onopen = () => {
        options?.onStatusChange?.("connecting");
        ws.send(
          JSON.stringify({
            jsonrpc: "2.0",
            id: 1,
            method: "subscribeEvents",
            params: {
              filters: [{ contractIds: [this.config.contractId] }],
              pagination: { cursor },
            },
          }),
        );
        resetHeartbeat();
      };

      ws.onmessage = (msg) => {
        if (!active) return;
        try {
          const data = JSON.parse((msg as any).data as string) as any;
          const rawEvents = data?.result?.events ?? [];
          if (!Array.isArray(rawEvents) || rawEvents.length === 0) return;
          retryCount = 0;
          options?.onStatusChange?.("active");
          resetHeartbeat();

          const processed: any[] = [];
          let latestCursor = cursor;
          for (const rawEvent of rawEvents) {
            const normalized = normalizeEvent(rawEvent);
            if (options?.eventName && normalized.name !== options.eventName) continue;
            const token = rawEvent.pagingToken ?? rawEvent.paging_token ?? normalized.pagingToken;
            if (token && (!latestCursor || token > latestCursor)) latestCursor = token;
            processed.push(normalized);
          }
          cursor = latestCursor;
          if (cursor) options?.cursorStore?.set?.(cursor);
          processed.forEach(callback);
        } catch {
          // ignore malformed frames
        }
      };

      ws.onerror = () => {
        try { ws.close(); } catch {}
      };

      ws.onclose = () => {
        this.eventWs = null;
        if (!active) return;
        scheduleReconnect();
      };

      return true;
    };

    const scheduleReconnect = () => {
      if (!active) return;
      retryCount++;
      options?.onStatusChange?.("reconnecting");
      if (retryCount > maxRetries) {
        active = false;
        options?.onStatusChange?.("closed");
        return;
      }
      const delay = Math.random() * Math.min(maxBackoffMs, baseBackoffMs * 2 ** (retryCount - 1));
      this.eventPollHandle = setTimeout(start, delay);
    };

    const start = () => {
      if (!active) return;
      cleanup();
      options?.onStatusChange?.("connecting");
      // Prefer WS if supported by provider; otherwise fall back to polling.
      if (!tryWebSocket()) {
        poll();
      }
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
        resetHeartbeat();

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
          if (cursor) options?.cursorStore?.set?.(cursor);

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
        if (options?.onError) options.onError(err);
        scheduleReconnect();
      }
    };

    start();

    return () => {
      active = false;
      options?.onStatusChange?.("closed");
      cleanup();
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

    // Static tiers derived from simulation
    const ceilDiv = (a: bigint, b: bigint) => (a + b - BigInt(1)) / b;
    const mulCeil = (v: bigint, num: bigint, den: bigint) => ceilDiv(v * num, den);

    return {
      base: base.toString(),
      source: "simulation-fallback",
      low: base.toString(),
      medium: mulCeil(base, BigInt(15), BigInt(10)).toString(),
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

  /**
   * Upload metadata to IPFS using configured provider.
   * Helper method that wraps the standalone uploadMetadataToIPFS function.
   * 
   * @param metadata JSON metadata object
   * @param config IPFS upload configuration
   * @returns CID and gateway URL
   */
  async uploadMetadataToIPFS(
    metadata: Record<string, unknown>,
    config: IpfsUploadConfig
  ) {
    return uploadMetadataToIPFS(metadata, config);
  }

  /**
   * Fetch metadata from IPFS with fallback gateways.
   * Helper method that wraps the standalone fetchMetadataFromIPFS function.
   * 
   * @param cid IPFS Content Identifier
   * @param gateways Optional custom gateway list
   * @returns Parsed metadata object
   */
  async fetchMetadataFromIPFS(
    cid: string,
    gateways?: string[]
  ) {
    return fetchMetadataFromIPFS(cid, gateways);
  }

  // ── Balance monitoring (#489) ──────────────────────────────────────────────

  /**
   * Fetch the current XLM and token balances held by the grant's smart contract.
   *
   * Queries the Horizon API for the contract account's balances (native XLM
   * and any classic asset trustlines) and returns a structured snapshot.
   *
   * @param grantId - The grant ID (used to verify the grant exists; funds are
   *   held by the shared contract account at `config.contractId`)
   * @returns Structured GrantBalances snapshot
   */
  async getGrantBalances(grantId: number): Promise<GrantBalances> {
    const account = await this.horizonServer.loadAccount(this.config.contractId);

    const rawBalances = account.balances as Horizon.HorizonApi.BalanceLine[];
    const balances: GrantBalance[] = rawBalances.map((b) => {
      const isNative = b.asset_type === "native";
      const raw = b.balance;
      const stroops = this._parseBalanceToStroops(raw);
      return {
        assetCode: isNative ? "XLM" : (b as Horizon.HorizonApi.BalanceLineAsset).asset_code,
        assetIssuer: isNative ? "" : (b as Horizon.HorizonApi.BalanceLineAsset).asset_issuer,
        isNative,
        rawBalance: raw,
        balanceStroops: stroops,
        formatted: this._formatStroops(stroops),
      };
    });

    balances.sort((a, b) => {
      if (a.isNative) return -1;
      if (b.isNative) return 1;
      return a.assetCode.localeCompare(b.assetCode);
    });

    return {
      grantId,
      contractAddress: this.config.contractId,
      balances,
      ledger: Number((account as any).last_modified_ledger ?? 0),
      fetchedAt: new Date(),
    };
  }

  /**
   * Subscribe to balance changes for a grant's contract account.
   *
   * Polls Horizon on a configurable interval and invokes `onChange` whenever
   * any balance in the snapshot differs from the previous one.
   *
   * @param grantId - Grant ID to monitor
   * @param options - Polling config and callbacks
   * @returns Cleanup function — call it to stop listening
   */
  listenToGrantBalanceChanges(
    grantId: number,
    options: BalanceChangeListenerOptions,
  ): () => void {
    const { pollInterval = 10_000, onChange, onError } = options;
    let previous: GrantBalances | null = null;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | null = null;

    const poll = async () => {
      if (stopped) return;
      try {
        const current = await this.getGrantBalances(grantId);
        if (this._hasBalanceChanged(previous, current)) {
          onChange(current, previous);
        }
        previous = current;
      } catch (err) {
        onError?.(err instanceof Error ? err : new Error(String(err)));
      }
      if (!stopped) {
        timer = setTimeout(poll, pollInterval);
      }
    };

    void poll();
    return () => {
      stopped = true;
      if (timer !== null) clearTimeout(timer);
    };
  }

  // ── Transaction history (#483) ────────────────────────────────────────────

  /**
   * Retrieve StellarGrants-related transaction history for a wallet address.
   *
   * Queries the Horizon API for all transactions sourced from `address` and
   * returns them as typed GrantHistoryRecord entries ready for dashboard display.
   *
   * @param address - Stellar account address (G…)
   * @param options - Pagination and ordering
   *
   * @example
   * ```typescript
   * const { records, nextCursor } = await sdk.getTransactionHistory("GABC...", { limit: 20 });
   * // Load next page:
   * const page2 = await sdk.getTransactionHistory("GABC...", { cursor: nextCursor });
   * ```
   */
  async getTransactionHistory(
    address: string,
    options: HistoryOptions = {},
  ): Promise<HistoryResult> {
    const limit = Math.min(options.limit ?? 50, 200);
    const order = options.order ?? "desc";

    let builder = this.horizonServer
      .transactions()
      .forAccount(address)
      .limit(limit)
      .order(order as "asc" | "desc");

    if (options.cursor) {
      builder = builder.cursor(options.cursor);
    }

    const page = await builder.call();
    return this._parseHistoryPage(page, limit);
  }

  /**
   * Retrieve all transactions related to a specific grant ID.
   *
   * Queries the Horizon API scoped to the contract account and filters records
   * whose memo matches the `grant:<grantId>` convention.
   *
   * @param grantId - Numeric grant ID
   * @param options - Pagination and ordering
   *
   * @example
   * ```typescript
   * const { records } = await sdk.getGrantHistory(42, { limit: 100 });
   * const funded = records.filter(r => r.operationType === "grant_fund");
   * ```
   */
  async getGrantHistory(
    grantId: number,
    options: HistoryOptions = {},
  ): Promise<HistoryResult> {
    const limit = Math.min(options.limit ?? 50, 200);
    const order = options.order ?? "desc";
    const grantIdStr = String(grantId);

    let builder = this.horizonServer
      .transactions()
      .forAccount(this.config.contractId)
      .limit(limit)
      .order(order as "asc" | "desc");

    if (options.cursor) {
      builder = builder.cursor(options.cursor);
    }

    const page = await builder.call();
    const { records: all, nextCursor } = this._parseHistoryPage(page, limit);

    const records = all.filter(
      (r) =>
        r.grantId === grantIdStr ||
        r.memo?.toLowerCase().includes(`grant:${grantIdStr}`),
    );

    return { records, nextCursor: records.length > 0 ? nextCursor : undefined };
  }

  // ── Private helpers ───────────────────────────────────────────────────────

  private _parseBalanceToStroops(raw: string): bigint {
    const [whole = "0", frac = ""] = raw.split(".");
    const paddedFrac = frac.padEnd(7, "0").slice(0, 7);
    return BigInt(whole) * 10_000_000n + BigInt(paddedFrac);
  }

  private _formatStroops(stroops: bigint): string {
    const whole = stroops / 10_000_000n;
    const frac = (stroops % 10_000_000n).toString().padStart(7, "0");
    return `${whole}.${frac}`;
  }

  private _hasBalanceChanged(
    previous: GrantBalances | null,
    current: GrantBalances,
  ): boolean {
    if (!previous) return true;
    if (previous.balances.length !== current.balances.length) return true;
    for (const curr of current.balances) {
      const prev = previous.balances.find(
        (b) => b.assetCode === curr.assetCode && b.assetIssuer === curr.assetIssuer,
      );
      if (!prev || prev.balanceStroops !== curr.balanceStroops) return true;
    }
    return false;
  }

  private static readonly FUNCTION_NAME_MAP: Record<string, GrantOperationType> = {
    grant_create: "grant_create",
    grant_fund: "grant_fund",
    grant_cancel: "grant_cancel",
    milestone_submit: "milestone_submit",
    milestone_approve: "milestone_approve",
    milestone_reject: "milestone_reject",
    milestone_payout: "milestone_payout",
    grant_withdraw: "grant_withdraw",
  };

  private _parseHistoryPage(
    page: { records: unknown[] },
    limit: number,
  ): HistoryResult {
    const rawRecords = page.records as Array<{
      hash: string;
      created_at: string;
      successful: boolean;
      source_account: string;
      fee_charged: string;
      memo?: string;
      paging_token: string;
    }>;

    const records: GrantHistoryRecord[] = rawRecords.slice(0, limit).map((tx) => {
      let operationType: GrantOperationType = "unknown_contract_call";
      let grantId: string | undefined;

      if (tx.memo) {
        const match = /grant:(\d+)/i.exec(tx.memo);
        if (match) grantId = match[1];

        const normalised = tx.memo.toLowerCase().replace(/-/g, "_");
        operationType =
          StellarGrantsSDK.FUNCTION_NAME_MAP[normalised] ?? "unknown_contract_call";
      }

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
    });

    const lastRecord = rawRecords[rawRecords.length - 1];
    const nextCursor = lastRecord?.paging_token;

    return { records, nextCursor };
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
    options?: WriteOptions,
  ): Promise<unknown> {
    try {
      const tx = await this.buildTx(method, args, options);
      const simulation = await this.server.simulateTransaction(tx);
      this.ensureSimulationSuccess(simulation);

      const minResourceFee = BigInt(simulation?.minResourceFee ?? "0");

      // Fee selection precedence: explicit fee > simulatedFee > priority(minResourceFee) > defaultFee
      const desiredFee =
        options?.fee ??
        options?.simulatedFee ??
        (simulation?.minResourceFee
          ? this.applyFeePriority(minResourceFee, options?.feePriority).toString()
          : undefined);

      const txForSending = desiredFee
        ? await this.buildTx(method, args, { ...options, simulatedFee: desiredFee })
        : tx;

      const prepared = await this.server.prepareTransaction(txForSending);

      if (options?.returnUnsignedXdr) {
        const id =
          (globalThis as any).crypto?.randomUUID?.() ??
          `pending_${Date.now()}_${Math.random().toString(16).slice(2)}`;
        const xdr = prepared.toXDR();
        this.pendingXdrStore.save(id, xdr);
        return { id, xdr };
      }

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
      // Keep these conservative defaults aligned with `estimateFees()` fallback tiers.
      case "low":    return base;
      case "high":   return base * BigInt(2);
      case "medium":
      default:       return (base * BigInt(15) + BigInt(9)) / BigInt(10);
    }
  }

  private async buildTx(
    method: string,
    args: xdr.ScVal[],
    options?: {
      feePriority?: "low" | "medium" | "high";
      fee?: string;
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
    const fee = options?.fee ?? options?.simulatedFee ?? this.config.defaultFee ?? "100";

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

  /**
   * Batch multiple contract calls into a single Stellar transaction.
   */
  batch(): BatchBuilder {
    return new BatchBuilder(this);
  }

  /**
   * Multi-sig helper: merge signatures from multiple signed XDRs.
   */
  combineSignatures(baseXdr: string, signaturesXdrs: string[]): string {
    return combineSignatures(baseXdr, signaturesXdrs, this.config.networkPassphrase);
  }

  // -------------------------------------------------------------------------
  // Batch internals (intentionally not part of the public API surface)
  // -------------------------------------------------------------------------

  async __simulateBatch(calls: BatchCall[], options?: Omit<BatchSendOptions, "waitForConfirmation" | "returnUnsignedXdr">): Promise<any> {
    try {
      const tx = await this.__buildBatchTx(calls, options);
      const simulation = await this.server.simulateTransaction(tx);
      this.ensureSimulationSuccess(simulation);

      // Best-effort: if the RPC returns per-op results, surface the failing index.
      const results = (simulation as any)?.results;
      if (Array.isArray(results)) {
        const idx = results.findIndex((r: any) => r?.error);
        if (idx >= 0) {
          throw new BatchOperationError(
            `Batch simulation failed at operation #${idx}`,
            {
              operationIndex: idx,
              method: calls[idx]?.method,
              label: calls[idx]?.label,
              details: results[idx],
            },
          );
        }
      }

      return simulation;
    } catch (error) {
      throw parseSorobanError(error);
    }
  }

  async __sendBatch(calls: BatchCall[], options?: BatchSendOptions): Promise<any> {
    try {
      const tx = await this.__buildBatchTx(calls, options);
      const simulation = await this.server.simulateTransaction(tx);
      this.ensureSimulationSuccess(simulation);

      const minResourceFee = BigInt(simulation?.minResourceFee ?? "0");
      const desiredFee =
        options?.fee ??
        options?.simulatedFee ??
        (simulation?.minResourceFee
          ? this.applyFeePriority(minResourceFee, options?.feePriority).toString()
          : undefined);

      const txForSending = desiredFee
        ? await this.__buildBatchTx(calls, { ...options, simulatedFee: desiredFee })
        : tx;

      const prepared = await this.server.prepareTransaction(txForSending);

      if (options?.returnUnsignedXdr) {
        const id =
          (globalThis as any).crypto?.randomUUID?.() ??
          `pending_${Date.now()}_${Math.random().toString(16).slice(2)}`;
        const xdr = prepared.toXDR();
        this.pendingXdrStore.save(id, xdr);
        return { id, xdr };
      }

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

  private async __buildBatchTx(
    calls: BatchCall[],
    options?: { feePriority?: FeePriority; fee?: string; simulatedFee?: string; footprint?: any },
  ): Promise<any> {
    const signer = this.config.signer;
    if (!signer) {
      throw new Error("A signer is required to build a transaction.");
    }

    const source = await signer.getPublicKey();
    const account = await this.server.getAccount(source);
    const fee = options?.fee ?? options?.simulatedFee ?? this.config.defaultFee ?? "100";

    let builder = new TransactionBuilder(account, {
      fee,
      networkPassphrase: this.config.networkPassphrase,
    });

    for (const c of calls) {
      builder = builder.addOperation(this.contract.call(c.method, ...c.args));
    }

    builder = builder.setTimeout(60);

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
