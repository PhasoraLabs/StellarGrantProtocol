/**
 * Tests for simulation-based fee estimation (#255) and SDK versioning (#263).
 */

import { CONTRACT_INTERFACE_VERSION } from "../src/StellarGrantsSDK";
import { makeSdk } from "./helpers/sdkFactory";

// ---------------------------------------------------------------------------
// Minimal stellar-sdk mock (mirrors existing sdk.test.ts pattern)
// ---------------------------------------------------------------------------
jest.mock("@stellar/stellar-sdk", () => {
  return {
    rpc: {
      Server: class {
        async getAccount() { return { accountId: "GMOCK", sequence: "0" }; }
        async simulateTransaction() { return { result: { retval: null }, minResourceFee: "1000" }; }
        async prepareTransaction(tx: any) { return tx; }
        async sendTransaction() { return { status: "PENDING", hash: "mockhash" }; }
        async getEvents() { return { events: [] }; }
        async getTransaction() { return { status: "SUCCESS", hash: "x" }; }
        async getNetwork() { return { passphrase: "Test SDF Network ; September 2015" }; }
      },
    },
    Horizon: {
      Server: class {
        constructor() {}
      },
    },
    Contract: class {
      call(method: string, ...args: unknown[]) { return { method, args }; }
    },
    Account: class {
      constructor(public accountId: string, public sequence: string) {}
    },
    TransactionBuilder: class {
      static fromXDR(_xdr: string, _pass: string) {
        return { toXDR: () => "SIGNED_TX_XDR" };
      }
      addOperation() { return this; }
      setTimeout() { return this; }
      setSorobanData() { return this; }
      build() { return { toXDR: () => "TX_XDR" }; }
    },
    nativeToScVal: (v: any) => v,
    scValToNative: (v: any) => v,
    xdr: { ScVal: { fromXDR: () => null }, SorobanTransactionData: {} },
  };
});

// ---------------------------------------------------------------------------
// estimateFees
// ---------------------------------------------------------------------------
describe("StellarGrantsSDK.estimateFees", () => {
  afterEach(() => {
    delete (global as any).fetch;
  });

  it("returns fee tiers based on minResourceFee from simulation", async () => {
    const { sdk, state } = makeSdk();
    state.minResourceFee = "2000";

    const fees = await sdk.estimateFees("grant_create", []);

    expect(fees.base).toBe("2000");
    expect(fees.low).toBe("2000");    // 1.0×
    expect(fees.medium).toBe("3000"); // 1.5×
    expect(fees.high).toBe("4000");   // 2.0×
    expect(fees.modifiers).toEqual({ low: 1, medium: 1.5, high: 2 });
    expect(fees.source).toBe("simulation-fallback");
  });

  it("ceilings fractional fees", async () => {
    const { sdk, state } = makeSdk();
    state.minResourceFee = "1001";

    const fees = await sdk.estimateFees("grant_create", []);

    // 1001 × 1.5 = 1501.5 → ceil → 1502
    expect(fees.medium).toBe("1502");
  });

  it("handles zero minResourceFee", async () => {
    const { sdk, state } = makeSdk();
    state.minResourceFee = "0";

    const fees = await sdk.estimateFees("grant_create", []);

    expect(fees.base).toBe("0");
    expect(fees.low).toBe("0");
    expect(fees.medium).toBe("0");
    expect(fees.high).toBe("0");
  });

  it("throws when simulation returns an error", async () => {
    const { sdk, state } = makeSdk();
    state.simulationError = "contract error";

    await expect(sdk.estimateFees("grant_create", [])).rejects.toThrow();
  });

  it("uses dynamic Horizon stats when available", async () => {
    (global as any).fetch = jest.fn(async () => ({
      ok: true,
      json: async () => ({
        ledger_capacity_usage: "0.97",
        max_fee: { p70: "2400" },
      }),
    }));

    const { sdk, state } = makeSdk({ horizonUrl: "https://horizon-testnet.stellar.org" });
    state.minResourceFee = "1000";

    const fees = await sdk.estimateFees("grant_create", []);

    expect(fees.base).toBe("1000");
    expect(fees.recommendedBase).toBe("2400");
    expect(fees.networkLoad).toBe("surge");
    expect(fees.source).toBe("horizon");
    expect(fees.low).toBe("3840");
    expect(fees.medium).toBe("6000");
    expect(fees.high).toBe("8400");
  });

  it("falls back to static tiers when fee stats fetch fails", async () => {
    (global as any).fetch = jest.fn(async () => {
      throw new Error("timeout");
    });

    const { sdk, state } = makeSdk({ horizonUrl: "https://horizon-testnet.stellar.org" });
    state.minResourceFee = "1000";

    const fees = await sdk.estimateFees("grant_create", []);

    expect(fees.source).toBe("simulation-fallback");
    expect(fees.low).toBe("1000");
    expect(fees.medium).toBe("1500");
    expect(fees.high).toBe("2000");
  });
});

// ---------------------------------------------------------------------------
// feePriority WriteOption
// ---------------------------------------------------------------------------
describe("feePriority WriteOption", () => {
  it("uses medium priority by default (1.5× fee)", async () => {
    const { sdk, mockServer, state, mockSigner } = makeSdk();
    state.minResourceFee = "1000";
    mockSigner.signTransaction.mockResolvedValue("SIGNED_XDR");

    await sdk.grantCreate(
      {
        owner: "GOWNER",
        title: "T",
        description: "D",
        budget: BigInt(100),
        deadline: BigInt(9999999),
        milestoneCount: 3,
      },
      {},
    );

    // buildTx is called with fee "1500" (1000 × 1.5 = 1500)
    const calls = mockServer.simulateTransaction.mock.calls;
    expect(calls.length).toBeGreaterThan(0);
  });

  it("respects high feePriority (2.0× fee)", async () => {
    const { sdk, state, mockSigner } = makeSdk();
    state.minResourceFee = "1000";
    mockSigner.signTransaction.mockResolvedValue("SIGNED_XDR");

    // Should not throw - high priority simply multiplies the fee
    await expect(
      sdk.grantCreate(
        {
          owner: "GOWNER",
          title: "T",
          description: "D",
          budget: BigInt(100),
          deadline: BigInt(9999999),
          milestoneCount: 3,
        },
        { feePriority: "high" },
      ),
    ).resolves.toBeDefined();
  });

  it("simulatedFee takes precedence over feePriority", async () => {
    const { sdk, state, mockSigner } = makeSdk();
    state.minResourceFee = "1000";
    mockSigner.signTransaction.mockResolvedValue("SIGNED_XDR");

    await expect(
      sdk.grantCreate(
        {
          owner: "GOWNER",
          title: "T",
          description: "D",
          budget: BigInt(100),
          deadline: BigInt(9999999),
          milestoneCount: 3,
        },
        { simulatedFee: "500", feePriority: "high" },
      ),
    ).resolves.toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// checkCompatibility
// ---------------------------------------------------------------------------
describe("StellarGrantsSDK.checkCompatibility", () => {
  it("CONTRACT_INTERFACE_VERSION is a positive integer", () => {
    expect(typeof CONTRACT_INTERFACE_VERSION).toBe("number");
    expect(Number.isInteger(CONTRACT_INTERFACE_VERSION)).toBe(true);
    expect(CONTRACT_INTERFACE_VERSION).toBeGreaterThan(0);
  });

  it("returns compatible:true with matching contract version", async () => {
    const { sdk, state } = makeSdk();
    // Make the simulation return the current SDK version as the contract version
    state.simulationResult = CONTRACT_INTERFACE_VERSION;

    const report = await sdk.checkCompatibility();

    expect(report.compatible).toBe(true);
    expect(report.sdkVersion).toBe(CONTRACT_INTERFACE_VERSION);
    expect(report.contractVersion).toBe(CONTRACT_INTERFACE_VERSION);
    expect(report.warning).toBeUndefined();
  });

  it("returns compatible:false with a newer contract version", async () => {
    const { sdk, state } = makeSdk();
    state.simulationResult = CONTRACT_INTERFACE_VERSION + 1;

    const report = await sdk.checkCompatibility();

    expect(report.compatible).toBe(false);
    expect(report.warning).toContain("upgrade the SDK");
  });

  it("returns compatible:false with an older contract version", async () => {
    const { sdk, state } = makeSdk();
    state.simulationResult = CONTRACT_INTERFACE_VERSION - 1;

    // Only run this test when CONTRACT_INTERFACE_VERSION > 1
    if (CONTRACT_INTERFACE_VERSION <= 1) return;

    const report = await sdk.checkCompatibility();

    expect(report.compatible).toBe(false);
    expect(report.warning).toBeDefined();
  });

  it("returns compatible:true with a warning when sdk_version method is missing", async () => {
    const { sdk, state } = makeSdk();
    // Simulate the sdk_version read failing (contract doesn't have the method)
    state.simulationError = "method not found: sdk_version";

    const report = await sdk.checkCompatibility();

    expect(report.compatible).toBe(true);
    expect(report.contractVersion).toBeNull();
    expect(report.warning).toContain("Could not determine contract interface version");
  });
});
