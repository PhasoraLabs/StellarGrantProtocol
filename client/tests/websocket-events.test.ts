import { makeSdk } from "./helpers/sdkFactory";

jest.mock("@stellar/stellar-sdk", () => {
  const actual = jest.requireActual("@stellar/stellar-sdk");
  return {
    ...actual,
    Contract: jest.fn().mockImplementation(() => ({
      call: jest.fn(),
    })),
  };
});

describe("WebSocket event subscription", () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("uses polling when RPC URL is http (not WebSocket)", async () => {
    const { sdk, mockServer } = makeSdk({ rpcUrl: "https://rpc.test.mock" });
    mockServer.getEvents = jest.fn().mockResolvedValue({ events: [] });

    const callback = jest.fn();
    const unsubscribe = sdk.subscribeToEvents(callback);

    await Promise.resolve(); // Allow async polling to start
    expect(mockServer.getEvents).toHaveBeenCalled();
    unsubscribe();
  });

  it("normalizes event payload shape", async () => {
    const { sdk, mockServer } = makeSdk({ rpcUrl: "https://rpc.test.mock" });
    const rawEvent = {
      id: "event-123",
      type: "contract",
      contractId: "CD...",
      topic: [],
      value: { _scval: "value" },
      ledger: 100,
      timestamp: 1234567890,
    };

    mockServer.getEvents = jest.fn().mockResolvedValue({ events: [rawEvent] });

    const callback = jest.fn();
    sdk.subscribeToEvents(callback);

    await Promise.resolve(); // Allow async polling to complete

    expect(callback).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "event-123",
        type: "contract",
        contractId: "CD...",
        ledger: 100,
        timestamp: 1234567890,
      })
    );
  });

  it("unsubscribes correctly from polling", async () => {
    const { sdk, mockServer } = makeSdk({ rpcUrl: "https://rpc.test.mock" });
    mockServer.getEvents = jest.fn().mockResolvedValue({ events: [] });

    const callback = jest.fn();
    const unsubscribe = sdk.subscribeToEvents(callback);

    await Promise.resolve();
    unsubscribe();
    jest.runAllTimers();

    // After unsubscribe, getEvents should not be called again
    expect(mockServer.getEvents).toHaveBeenCalledTimes(1);
  });

  // ── WebSocket path ──────────────────────────────────────────────────────

  it("uses WebSocket when wss:// URL is provided", async () => {
    const mockSocket = {
      onmessage: null as ((e: any) => void) | null,
      onerror:   null as ((e: any) => void) | null,
      onclose:   null as ((e: any) => void) | null,
      close:     jest.fn(),
    };
    (globalThis as any).WebSocket = jest.fn(() => mockSocket);

    const { sdk } = makeSdk({ rpcUrl: "wss://rpc.test.mock" });
    const callback = jest.fn();
    const unsubscribe = sdk.subscribeToEvents(callback);

    const rawEvent = { id: "ws-1", type: "contract", contractId: "CD...", topic: [], value: null, ledger: 10, timestamp: 999 };
    mockSocket.onmessage?.({ data: JSON.stringify({ events: [rawEvent] }) });

    expect(callback).toHaveBeenCalledWith(
      expect.objectContaining({ id: "ws-1", ledger: 10, timestamp: 999 }),
    );

    unsubscribe();
    expect(mockSocket.close).toHaveBeenCalled();

    delete (globalThis as any).WebSocket;
  });

  it("falls back to polling when WebSocket triggers an error", async () => {
    const mockSocket = {
      onmessage: null as ((e: any) => void) | null,
      onerror:   null as ((e: any) => void) | null,
      onclose:   null as ((e: any) => void) | null,
      close:     jest.fn(),
    };
    (globalThis as any).WebSocket = jest.fn(() => mockSocket);

    const { sdk, mockServer } = makeSdk({ rpcUrl: "wss://rpc.test.mock" });
    mockServer.getEvents = jest.fn().mockResolvedValue({ events: [] });

    const callback = jest.fn();
    const unsubscribe = sdk.subscribeToEvents(callback);

    // Trigger socket error → should fall back to polling
    mockSocket.onerror?.({});
    await Promise.resolve();

    expect(mockServer.getEvents).toHaveBeenCalled();
    unsubscribe();
    delete (globalThis as any).WebSocket;
  });

  it("falls back to polling when WebSocket closes unexpectedly", async () => {
    const mockSocket = {
      onmessage: null as ((e: any) => void) | null,
      onerror:   null as ((e: any) => void) | null,
      onclose:   null as ((e: any) => void) | null,
      close:     jest.fn(),
    };
    (globalThis as any).WebSocket = jest.fn(() => mockSocket);

    const { sdk, mockServer } = makeSdk({ rpcUrl: "wss://rpc.test.mock" });
    mockServer.getEvents = jest.fn().mockResolvedValue({ events: [] });

    const callback = jest.fn();
    const unsubscribe = sdk.subscribeToEvents(callback);

    // Simulate an unexpected close event
    mockSocket.onclose?.({});
    await Promise.resolve();

    expect(mockServer.getEvents).toHaveBeenCalled();
    unsubscribe();
    delete (globalThis as any).WebSocket;
  });

  it("normalizes WebSocket event payload to the same shape as polling", async () => {
    const mockSocket = {
      onmessage: null as ((e: any) => void) | null,
      onerror:   null as ((e: any) => void) | null,
      onclose:   null as ((e: any) => void) | null,
      close:     jest.fn(),
    };
    (globalThis as any).WebSocket = jest.fn(() => mockSocket);

    const { sdk } = makeSdk({ rpcUrl: "wss://rpc.test.mock" });
    const callback = jest.fn();
    sdk.subscribeToEvents(callback);

    const frame = {
      id: "ev-2",
      type: "contract",
      contract_id: "CXYZ",   // snake_case variant
      topic: ["sym"],
      value: { amount: 100 },
      ledger: 42,
      timestamp: 12345,
    };
    mockSocket.onmessage?.({ data: JSON.stringify({ events: [frame] }) });

    expect(callback).toHaveBeenCalledWith(
      expect.objectContaining({
        contractId: "CXYZ",
        ledger: 42,
        value: { amount: 100 },
      }),
    );

    delete (globalThis as any).WebSocket;
  });
});
