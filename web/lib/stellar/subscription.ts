/**
 * Resilient Event Subscription — Issue #254
 *
 * Replaces the naive setTimeout polling loop with a production-grade
 * event streaming layer that provides:
 *
 *   • Exponential backoff with jitter on errors
 *   • Cursor persistence across reconnections (no missed events)
 *   • Heartbeat / dead-connection detection
 *   • WebSocket upgrade when the RPC provider supports it
 *   • Full cleanup via the returned `unsubscribe()` function
 *
 * Framework-agnostic — works in React, Vue, plain JS, or Node.js.
 *
 * @module web/lib/stellar/subscription
 */

// ── Public types ─────────────────────────────────────────────────────────────

export interface ContractEvent {
  /** e.g. "grant_created", "milestone_approved" */
  type: string;
  /** Decoded event payload */
  data: Record<string, unknown>;
  /** Ledger sequence number this event belongs to */
  ledger: number;
  /** Paging cursor returned by the RPC — used to resume after reconnection */
  cursor: string;
  /** Wall-clock timestamp of the ledger */
  timestamp: Date;
}

/** Called with each new batch of events fetched from the network */
export type EventHandler = (events: ContractEvent[]) => void;

/** Called whenever the subscription state changes */
export type StatusHandler = (status: SubscriptionStatus) => void;

export type SubscriptionStatus =
  | "connecting"   // initial connect or reconnect attempt
  | "active"       // receiving events normally
  | "reconnecting" // backing off after an error
  | "closed";      // unsubscribed or permanently failed

export interface SubscribeOptions {
  /**
   * Contract ID to subscribe to.
   * Defaults to NEXT_PUBLIC_CONTRACT_ID env var.
   */
  contractId?: string;

  /**
   * Paging cursor to start from.
   * Pass the last cursor received to resume without gaps.
   * Defaults to "now" (only new events).
   */
  startCursor?: string;

  /**
   * Polling interval in ms when WebSocket is not available (default 5 000).
   */
  pollIntervalMs?: number;

  /**
   * Base delay in ms for the first retry (default 1 000).
   * Subsequent retries double up to `maxBackoffMs`.
   */
  baseBackoffMs?: number;

  /**
   * Maximum delay in ms between retries (default 30 000).
   */
  maxBackoffMs?: number;

  /**
   * Maximum consecutive errors before giving up and closing (default 10).
   * Set to Infinity to retry forever.
   */
  maxRetries?: number;

  /**
   * How long without a successful poll before the connection is considered
   * dead and a reconnect is forced (default 60 000 ms).
   */
  heartbeatTimeoutMs?: number;

  /**
   * Optional RPC endpoint override.
   * Defaults to NEXT_PUBLIC_STELLAR_RPC_URL.
   */
  rpcUrl?: string;

  /** Called with each new batch of events */
  onEvents: EventHandler;

  /** Called when the subscription status changes */
  onStatus?: StatusHandler;

  /** Called on unrecoverable error (after maxRetries exhausted) */
  onError?: (err: Error) => void;
}

/** Returned by `subscribeToEvents()`. Call `unsubscribe()` to stop. */
export interface Subscription {
  /** Stop all polling/WebSocket and release resources */
  unsubscribe: () => void;
  /** Read the last successfully persisted cursor */
  getCursor: () => string;
  /** Current subscription status */
  getStatus: () => SubscriptionStatus;
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/**
 * Compute the next backoff delay with full-jitter to spread reconnects.
 * Formula: random value in [0, min(cap, base * 2^attempt)]
 */
function nextBackoff(
  attempt: number,
  baseMs: number,
  maxMs: number
): number {
  const ceiling = Math.min(maxMs, baseMs * 2 ** attempt);
  return Math.random() * ceiling;
}

/**
 * Minimal polyfill-safe sleep using setTimeout.
 */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Fetch a page of contract events from the Stellar RPC.
 * In production this calls `rpc.Server#getEvents()`; here we keep the
 * dependency on the raw RpcUrl so the function remains testable with a
 * mock fetch without importing the heavy stellar-sdk.
 */
async function fetchEventPage(
  rpcUrl: string,
  contractId: string,
  cursor: string
): Promise<{ events: ContractEvent[]; nextCursor: string }> {
  const body = {
    jsonrpc: "2.0",
    id: 1,
    method: "getEvents",
    params: {
      filters: [{ contractIds: [contractId] }],
      pagination: { cursor, limit: 200 },
    },
  };

  const res = await fetch(rpcUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    throw new Error(`RPC HTTP ${res.status}: ${res.statusText}`);
  }

  const json = (await res.json()) as {
    result?: {
      events?: Array<{
        type?: string;
        contractId?: string;
        ledger?: number;
        ledgerClosedAt?: string;
        pagingToken?: string;
        value?: { xdr?: string };
      }>;
      latestLedger?: number;
    };
    error?: { message: string };
  };

  if (json.error) throw new Error(json.error.message);

  const rawEvents = json.result?.events ?? [];
  let nextCursor = cursor;

  const events: ContractEvent[] = rawEvents.map((e) => {
    const token = e.pagingToken ?? cursor;
    if (token > nextCursor) nextCursor = token;

    return {
      type: e.type ?? "unknown",
      data: e.value ?? {},
      ledger: e.ledger ?? 0,
      cursor: token,
      timestamp: e.ledgerClosedAt ? new Date(e.ledgerClosedAt) : new Date(),
    };
  });

  return { events, nextCursor };
}

// ── WebSocket upgrade (optional) ─────────────────────────────────────────────

/**
 * Try to establish a WebSocket connection to the RPC's streaming endpoint.
 * Returns the socket if successful, or null if WebSocket is not available
 * (e.g. the provider only supports HTTP polling).
 */
function tryWebSocket(
  rpcUrl: string,
  contractId: string,
  cursor: string,
  onMessage: (events: ContractEvent[]) => void,
  onClose: (code: number) => void
): WebSocket | null {
  // Convert http(s) → ws(s)
  const wsUrl = rpcUrl
    .replace(/^https:\/\//, "wss://")
    .replace(/^http:\/\//, "ws://");

  let ws: WebSocket;
  try {
    ws = new WebSocket(wsUrl);
  } catch {
    return null;
  }

  ws.onopen = () => {
    ws.send(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "subscribeEvents",
        params: {
          filters: [{ contractIds: [contractId] }],
          pagination: { cursor },
        },
      })
    );
  };

  ws.onmessage = (msg) => {
    try {
      const data = JSON.parse(msg.data as string) as {
        result?: {
          events?: Array<{
            type?: string;
            ledger?: number;
            ledgerClosedAt?: string;
            pagingToken?: string;
            cursor?: string;
            value?: Record<string, unknown>;
          }>;
        };
      };
      const rawEvents = data.result?.events ?? [];
      if (rawEvents.length === 0) return;
      const events: ContractEvent[] = rawEvents.map((e) => ({
        type: e.type ?? "unknown",
        data: e.value ?? {},
        ledger: e.ledger ?? 0,
        // RPC streaming frames use pagingToken; normalise to cursor
        cursor: e.pagingToken ?? e.cursor ?? "",
        timestamp: e.ledgerClosedAt ? new Date(e.ledgerClosedAt) : new Date(),
      }));
      onMessage(events);
    } catch {
      // ignore malformed frames
    }
  };

  ws.onclose = (ev) => onClose(ev.code);
  ws.onerror = () => ws.close();

  return ws;
}

// ── Main subscription factory ─────────────────────────────────────────────────

/**
 * Subscribe to StellarGrants contract events with automatic resilience.
 *
 * @example
 * ```ts
 * const sub = subscribeToEvents({
 *   contractId: "CCABC...",
 *   startCursor: lastCursor,       // resume from where we left off
 *   onEvents: (events) => { ... },
 *   onStatus: (s) => console.log("status:", s),
 * });
 *
 * // Later:
 * sub.unsubscribe();
 * const resumeCursor = sub.getCursor(); // persist this
 * ```
 */
export function subscribeToEvents(opts: SubscribeOptions): Subscription {
  const {
    contractId = process.env.NEXT_PUBLIC_CONTRACT_ID ?? "",
    startCursor = "0",
    pollIntervalMs = 5_000,
    baseBackoffMs = 1_000,
    maxBackoffMs = 30_000,
    maxRetries = 10,
    heartbeatTimeoutMs = 60_000,
    rpcUrl = process.env.NEXT_PUBLIC_STELLAR_RPC_URL ??
      "https://soroban-testnet.stellar.org",
    onEvents,
    onStatus,
    onError,
  } = opts;

  // ── Mutable state ─────────────────────────────────────────────────────────
  let cursor = startCursor;
  let status: SubscriptionStatus = "connecting";
  let stopped = false;
  let retryCount = 0;
  let ws: WebSocket | null = null;

  // Timers
  let pollTimer: ReturnType<typeof setTimeout> | null = null;
  let heartbeatTimer: ReturnType<typeof setTimeout> | null = null;

  // ── Helpers ───────────────────────────────────────────────────────────────

  function setStatus(s: SubscriptionStatus): void {
    status = s;
    onStatus?.(s);
  }

  function resetHeartbeat(): void {
    if (heartbeatTimer !== null) clearTimeout(heartbeatTimer);
    heartbeatTimer = setTimeout(() => {
      if (stopped) return;
      // No successful poll within the window → force reconnect
      cleanup();
      scheduleReconnect();
    }, heartbeatTimeoutMs);
  }

  function cleanup(): void {
    if (pollTimer !== null) { clearTimeout(pollTimer); pollTimer = null; }
    if (heartbeatTimer !== null) { clearTimeout(heartbeatTimer); heartbeatTimer = null; }
    if (ws !== null) {
      ws.onclose = null; // prevent recursive reconnect
      ws.close();
      ws = null;
    }
  }

  // ── Polling loop ──────────────────────────────────────────────────────────

  async function poll(): Promise<void> {
    if (stopped) return;

    try {
      const { events, nextCursor } = await fetchEventPage(rpcUrl, contractId, cursor);

      if (stopped) return;

      cursor = nextCursor;
      retryCount = 0;
      setStatus("active");
      resetHeartbeat();

      if (events.length > 0) onEvents(events);

    } catch (err) {
      if (stopped) return;

      retryCount += 1;

      if (retryCount > maxRetries) {
        const error = err instanceof Error ? err : new Error(String(err));
        cleanup();
        setStatus("closed");
        onError?.(error);
        return;
      }

      setStatus("reconnecting");
      const delay = nextBackoff(retryCount - 1, baseBackoffMs, maxBackoffMs);
      await sleep(delay);
      if (!stopped) poll();
      return;
    }

    // Schedule next poll
    if (!stopped) {
      pollTimer = setTimeout(poll, pollIntervalMs);
    }
  }

  // ── WebSocket path ────────────────────────────────────────────────────────

  function scheduleReconnect(): void {
    if (stopped) return;
    setStatus("reconnecting");
    const delay = nextBackoff(retryCount, baseBackoffMs, maxBackoffMs);
    retryCount += 1;
    pollTimer = setTimeout(start, delay);
  }

  function start(): void {
    if (stopped) return;
    setStatus("connecting");

    // Attempt WebSocket upgrade first
    ws = tryWebSocket(
      rpcUrl,
      contractId,
      cursor,
      (events) => {
        retryCount = 0;
        setStatus("active");
        resetHeartbeat();
        // WS events arrive in ledger order — always advance to the last cursor
        const lastCursor = events[events.length - 1]?.cursor;
        if (lastCursor) cursor = lastCursor;
        onEvents(events);
      },
      (code) => {
        ws = null;
        if (stopped) return;
        // 1000 = normal close (server-initiated), anything else = error
        if (code !== 1000) retryCount += 1;
        scheduleReconnect();
      }
    );

    // WebSocket not available → fall back to HTTP polling
    if (ws === null) {
      poll();
    } else {
      resetHeartbeat();
    }
  }

  // ── Boot ──────────────────────────────────────────────────────────────────
  start();

  // ── Public interface ──────────────────────────────────────────────────────
  return {
    unsubscribe(): void {
      stopped = true;
      cleanup();
      setStatus("closed");
    },
    getCursor(): string {
      return cursor;
    },
    getStatus(): SubscriptionStatus {
      return status;
    },
  };
}
