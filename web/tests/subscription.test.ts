/**
 * Resilient Event Subscription Tests — Issue #254
 *
 * Uses vi.useFakeTimers() + vi.advanceTimersByTimeAsync() to advance
 * time by bounded amounts, avoiding the infinite-loop trap that
 * vi.runAllTimersAsync() causes with recurring poll loops.
 *
 * fetch() and WebSocket are fully mocked — no live network required.
 */

import {
  describe,
  it,
  expect,
  vi,
  beforeEach,
  afterEach,
  type MockInstance,
} from "vitest";
import { subscribeToEvents } from "../lib/stellar/subscription";
import type { ContractEvent, SubscriptionStatus } from "../lib/stellar/subscription";

// ── RPC mock helpers ─────────────────────────────────────────────────────────

function rpcResponse(events: Partial<ContractEvent>[] = []) {
  return {
    ok: true,
    json: async () => ({
      result: {
        events: events.map((e, i) => ({
          type: e.type ?? "test_event",
          ledger: e.ledger ?? 100 + i,
          ledgerClosedAt: new Date().toISOString(),
          pagingToken: e.cursor ?? `cursor-${i}`,
          value: e.data ?? {},
        })),
        latestLedger: 9999,
      },
    }),
  };
}

function rpcError(status = 500, text = "Internal Server Error") {
  return { ok: false, status, statusText: text, json: async () => ({}) };
}

function rpcJsonError(message: string) {
  return {
    ok: true,
    json: async () => ({ error: { message } }),
  };
}

// ── WebSocket mock ────────────────────────────────────────────────────────────

class MockWebSocket {
  static instances: MockWebSocket[] = [];

  readyState = 0;
  onopen: (() => void) | null = null;
  onmessage: ((ev: { data: string }) => void) | null = null;
  onclose: ((ev: { code: number }) => void) | null = null;
  onerror: (() => void) | null = null;
  sentMessages: string[] = [];

  constructor(public url: string) {
    MockWebSocket.instances.push(this);
  }

  send(msg: string) {
    this.sentMessages.push(msg);
  }

  close(code = 1000) {
    this.readyState = 3;
    this.onclose?.({ code });
  }

  simulateOpen() {
    this.readyState = 1;
    this.onopen?.();
  }

  simulateMessage(events: Partial<ContractEvent>[]) {
    const payload = JSON.stringify({
      result: {
        events: events.map((e, i) => ({
          type: e.type ?? "ws_event",
          ledger: e.ledger ?? 200 + i,
          pagingToken: e.cursor ?? `ws-cursor-${i}`,
          value: e.data ?? {},
        })),
      },
    });
    this.onmessage?.({ data: payload });
  }

  simulateDrop(code = 1006) {
    this.readyState = 3;
    this.onerror?.();
    this.onclose?.({ code });
  }
}

// ── Setup / teardown ─────────────────────────────────────────────────────────

let fetchMock: MockInstance;

beforeEach(() => {
  vi.useFakeTimers();
  MockWebSocket.instances = [];
  // Default: WebSocket throws → forces HTTP polling path
  vi.stubGlobal("WebSocket", () => {
    throw new Error("no ws");
  });

  fetchMock = vi
    .spyOn(global, "fetch")
    .mockResolvedValue(rpcResponse() as unknown as Response);
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

// ── HTTP polling path ─────────────────────────────────────────────────────────

describe("subscribeToEvents — HTTP polling path", () => {
  it("calls onEvents with events from the first poll", async () => {
    const onEvents = vi.fn();
    fetchMock.mockResolvedValue(
      rpcResponse([{ type: "grant_created", cursor: "c1" }]) as unknown as Response
    );

    const sub = subscribeToEvents({
      onEvents,
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 1_000,
    });

    // Let the first fetch promise resolve, then the callback fires
    await vi.advanceTimersByTimeAsync(100);

    expect(onEvents).toHaveBeenCalledOnce();
    const [events] = onEvents.mock.calls[0] as [ContractEvent[]];
    expect(events[0].type).toBe("grant_created");
    sub.unsubscribe();
  });

  it("advances cursor after the first successful poll", async () => {
    fetchMock.mockResolvedValue(
      rpcResponse([{ cursor: "c-42" }]) as unknown as Response
    );

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 1_000,
    });

    await vi.advanceTimersByTimeAsync(100);
    expect(sub.getCursor()).toBe("c-42");
    sub.unsubscribe();
  });

  it("schedules a second poll after pollIntervalMs", async () => {
    fetchMock.mockResolvedValue(rpcResponse() as unknown as Response);

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 500,
    });

    // first poll fires immediately
    await vi.advanceTimersByTimeAsync(100);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    // second poll after interval
    await vi.advanceTimersByTimeAsync(600);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    sub.unsubscribe();
  });

  it("does not call onEvents when the event list is empty", async () => {
    fetchMock.mockResolvedValue(rpcResponse([]) as unknown as Response);
    const onEvents = vi.fn();

    const sub = subscribeToEvents({
      onEvents,
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 1_000,
    });

    await vi.advanceTimersByTimeAsync(100);
    expect(onEvents).not.toHaveBeenCalled();
    sub.unsubscribe();
  });

  it("sets status to 'active' after a successful poll", async () => {
    const statuses: SubscriptionStatus[] = [];
    fetchMock.mockResolvedValue(rpcResponse() as unknown as Response);

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      onStatus: (s) => statuses.push(s),
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 1_000,
    });

    await vi.advanceTimersByTimeAsync(100);
    expect(statuses).toContain("active");
    sub.unsubscribe();
  });
});

// ── Exponential backoff ───────────────────────────────────────────────────────

describe("subscribeToEvents — exponential backoff", () => {
  it("retries after an HTTP error and eventually succeeds", async () => {
    const onEvents = vi.fn();
    const statuses: SubscriptionStatus[] = [];

    fetchMock
      .mockResolvedValueOnce(rpcError() as unknown as Response)
      .mockResolvedValueOnce(rpcError() as unknown as Response)
      .mockResolvedValue(
        rpcResponse([{ type: "grant_funded" }]) as unknown as Response
      );

    const sub = subscribeToEvents({
      onEvents,
      onStatus: (s) => statuses.push(s),
      contractId: "C1",
      rpcUrl: "http://rpc",
      baseBackoffMs: 10,
      maxBackoffMs: 50,
      pollIntervalMs: 1_000,
    });

    // First call fails → reconnecting → backoff (~0–10ms)
    await vi.advanceTimersByTimeAsync(20);
    // Second call fails → reconnecting
    await vi.advanceTimersByTimeAsync(50);
    // Third call succeeds
    await vi.advanceTimersByTimeAsync(100);

    expect(statuses).toContain("reconnecting");
    expect(statuses).toContain("active");
    expect(onEvents).toHaveBeenCalled();
    sub.unsubscribe();
  });

  it("retries after a JSON-level RPC error", async () => {
    const onEvents = vi.fn();

    fetchMock
      .mockResolvedValueOnce(
        rpcJsonError("ledger not ready") as unknown as Response
      )
      .mockResolvedValue(
        rpcResponse([{ type: "ok" }]) as unknown as Response
      );

    const sub = subscribeToEvents({
      onEvents,
      contractId: "C1",
      rpcUrl: "http://rpc",
      baseBackoffMs: 10,
      maxBackoffMs: 50,
      pollIntervalMs: 1_000,
    });

    await vi.advanceTimersByTimeAsync(200);
    expect(onEvents).toHaveBeenCalled();
    sub.unsubscribe();
  });

  it("calls onError and closes when maxRetries is exhausted", async () => {
    const onError = vi.fn();
    const statuses: SubscriptionStatus[] = [];

    fetchMock.mockResolvedValue(rpcError(503) as unknown as Response);

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      onStatus: (s) => statuses.push(s),
      onError,
      contractId: "C1",
      rpcUrl: "http://rpc",
      maxRetries: 3,
      baseBackoffMs: 10,
      maxBackoffMs: 50,
      pollIntervalMs: 1_000,
    });

    // Advance enough for 4 attempts (1 + 3 retries) + their backoffs
    await vi.advanceTimersByTimeAsync(500);

    expect(onError).toHaveBeenCalledOnce();
    expect(statuses[statuses.length - 1]).toBe("closed");
    sub.unsubscribe();
  });

  it("does not retry after unsubscribe is called", async () => {
    fetchMock.mockResolvedValue(rpcError() as unknown as Response);
    const onError = vi.fn();

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      onError,
      contractId: "C1",
      rpcUrl: "http://rpc",
      baseBackoffMs: 50,
      maxRetries: 5,
      pollIntervalMs: 1_000,
    });

    sub.unsubscribe();
    await vi.advanceTimersByTimeAsync(500);

    expect(onError).not.toHaveBeenCalled();
  });
});

// ── Cursor persistence ────────────────────────────────────────────────────────

describe("subscribeToEvents — cursor management", () => {
  it("starts from the provided startCursor", async () => {
    fetchMock.mockResolvedValue(rpcResponse() as unknown as Response);

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      contractId: "C1",
      rpcUrl: "http://rpc",
      startCursor: "ledger-999",
      pollIntervalMs: 1_000,
    });

    await vi.advanceTimersByTimeAsync(100);

    const body = JSON.parse(
      (fetchMock.mock.calls[0][1] as RequestInit).body as string
    ) as { params: { pagination: { cursor: string } } };

    expect(body.params.pagination.cursor).toBe("ledger-999");
    sub.unsubscribe();
  });

  it("uses latest event cursor in the second poll request", async () => {
    fetchMock
      .mockResolvedValueOnce(
        rpcResponse([{ cursor: "ledger-200" }]) as unknown as Response
      )
      .mockResolvedValue(rpcResponse() as unknown as Response);

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 200,
    });

    // First poll
    await vi.advanceTimersByTimeAsync(100);
    expect(sub.getCursor()).toBe("ledger-200");

    // Second poll fires after interval
    await vi.advanceTimersByTimeAsync(300);

    const secondBody = JSON.parse(
      (fetchMock.mock.calls[1][1] as RequestInit).body as string
    ) as { params: { pagination: { cursor: string } } };

    expect(secondBody.params.pagination.cursor).toBe("ledger-200");
    sub.unsubscribe();
  });

  it("preserves cursor across a failed poll", async () => {
    fetchMock
      .mockResolvedValueOnce(
        rpcResponse([{ cursor: "c-100" }]) as unknown as Response
      )
      .mockResolvedValueOnce(rpcError() as unknown as Response)
      .mockResolvedValue(rpcResponse() as unknown as Response);

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 100,
      baseBackoffMs: 10,
      maxBackoffMs: 50,
    });

    // First poll succeeds, sets cursor
    await vi.advanceTimersByTimeAsync(100);
    expect(sub.getCursor()).toBe("c-100");

    // Second poll fails, cursor must not change
    await vi.advanceTimersByTimeAsync(200);
    expect(sub.getCursor()).toBe("c-100");

    sub.unsubscribe();
  });
});

// ── Unsubscribe / cleanup ────────────────────────────────────────────────────

describe("subscribeToEvents — unsubscribe", () => {
  it("sets status to 'closed' immediately on unsubscribe", async () => {
    fetchMock.mockResolvedValue(rpcResponse() as unknown as Response);
    const statuses: SubscriptionStatus[] = [];

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      onStatus: (s) => statuses.push(s),
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 1_000,
    });

    sub.unsubscribe();
    expect(sub.getStatus()).toBe("closed");
    expect(statuses[statuses.length - 1]).toBe("closed");
  });

  it("stops calling onEvents after unsubscribe", async () => {
    fetchMock.mockResolvedValue(
      rpcResponse([{ type: "ev" }]) as unknown as Response
    );
    const onEvents = vi.fn();

    const sub = subscribeToEvents({
      onEvents,
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 200,
    });

    // Let first poll complete
    await vi.advanceTimersByTimeAsync(100);
    const countBefore = onEvents.mock.calls.length;

    sub.unsubscribe();

    // Advance well past the next poll interval — should not fire again
    await vi.advanceTimersByTimeAsync(2_000);
    expect(onEvents.mock.calls.length).toBe(countBefore);
  });
});

// ── Heartbeat ────────────────────────────────────────────────────────────────

describe("subscribeToEvents — heartbeat", () => {
  it("forces reconnect when no successful poll arrives within heartbeatTimeoutMs", async () => {
    let callCount = 0;
    fetchMock.mockImplementation(async () => {
      callCount += 1;
      if (callCount === 1) {
        return rpcResponse([{ cursor: "c1" }]) as unknown as Response;
      }
      // Hang forever on subsequent calls
      return new Promise<Response>(() => { /* never resolves */ });
    });

    const statuses: SubscriptionStatus[] = [];

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      onStatus: (s) => statuses.push(s),
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 200,
      heartbeatTimeoutMs: 500,
      baseBackoffMs: 10,
    });

    // First poll succeeds
    await vi.advanceTimersByTimeAsync(100);
    expect(statuses).toContain("active");

    // Advance past heartbeat window without another success
    await vi.advanceTimersByTimeAsync(600);
    expect(statuses).toContain("reconnecting");

    sub.unsubscribe();
  });
});

// ── WebSocket upgrade ────────────────────────────────────────────────────────

describe("subscribeToEvents — WebSocket upgrade", () => {
  beforeEach(() => {
    vi.stubGlobal("WebSocket", MockWebSocket);
  });

  it("uses WebSocket when available and emits events on message", async () => {
    const onEvents = vi.fn();
    const statuses: SubscriptionStatus[] = [];

    const sub = subscribeToEvents({
      onEvents,
      onStatus: (s) => statuses.push(s),
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 1_000,
    });

    await vi.advanceTimersByTimeAsync(10);
    const ws = MockWebSocket.instances[0];
    expect(ws).toBeDefined();

    ws.simulateOpen();
    ws.simulateMessage([{ type: "grant_created", cursor: "ws-c1" }]);

    expect(onEvents).toHaveBeenCalledOnce();
    expect(statuses).toContain("active");
    sub.unsubscribe();
  });

  it("sends subscription message with contractId and startCursor on open", async () => {
    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      contractId: "CONTRACT_XYZ",
      rpcUrl: "http://rpc",
      startCursor: "cursor-start",
      pollIntervalMs: 1_000,
    });

    await vi.advanceTimersByTimeAsync(10);
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    const msg = JSON.parse(ws.sentMessages[0]) as {
      params: {
        filters: Array<{ contractIds: string[] }>;
        pagination: { cursor: string };
      };
    };

    expect(msg.params.filters[0].contractIds).toContain("CONTRACT_XYZ");
    expect(msg.params.pagination.cursor).toBe("cursor-start");
    sub.unsubscribe();
  });

  it("falls back to HTTP polling when WebSocket drops unexpectedly", async () => {
    fetchMock.mockResolvedValue(
      rpcResponse([{ type: "polled" }]) as unknown as Response
    );

    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      contractId: "C1",
      rpcUrl: "http://rpc",
      baseBackoffMs: 10,
      pollIntervalMs: 1_000,
    });

    await vi.advanceTimersByTimeAsync(10);
    const ws = MockWebSocket.instances[0];

    // Remove WS stub so the next start() attempt can't create a WS → polls instead
    vi.stubGlobal("WebSocket", () => { throw new Error("no ws"); });
    ws.simulateDrop(1006);

    // After drop + backoff, falls back to HTTP polling
    await vi.advanceTimersByTimeAsync(100);
    expect(fetchMock).toHaveBeenCalled();
    sub.unsubscribe();
  });

  it("advances the cursor from WebSocket events", async () => {
    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      contractId: "C1",
      rpcUrl: "http://rpc",
      startCursor: "ws-start",
      pollIntervalMs: 1_000,
    });

    await vi.advanceTimersByTimeAsync(10);
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();
    // simulateMessage encodes cursor under pagingToken (matches production RPC format)
    ws.simulateMessage([{ type: "grant_created", cursor: "ws-cursor-99" }]);

    expect(sub.getCursor()).toBe("ws-cursor-99");
    sub.unsubscribe();
  });

  it("closes WebSocket cleanly on unsubscribe", async () => {
    const sub = subscribeToEvents({
      onEvents: vi.fn(),
      contractId: "C1",
      rpcUrl: "http://rpc",
      pollIntervalMs: 1_000,
    });

    await vi.advanceTimersByTimeAsync(10);
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    sub.unsubscribe();
    expect(sub.getStatus()).toBe("closed");
  });
});
