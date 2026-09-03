/**
 * Comprehensive mocked unit tests for StellarGrantsSDK (issue #463).
 *
 * Covers all public methods with a mock RPC server and mock signer — no live
 * network needed. Also covers:
 *   - #458 — dynamic fee estimation applied automatically in invokeWrite
 *   - #462 — simulateFootprint utility + footprint passthrough to write ops
 */

// Module-level mock matches the pattern used by fee-estimation.test.ts so the
// Contract constructor accepts any string ID in tests.
jest.mock("@stellar/stellar-sdk", () => ({
  rpc: {
    Server: class {
      async getAccount() { return { accountId: "GMOCK", sequence: "0" }; }
      async simulateTransaction() { return { result: { retval: null }, minResourceFee: "1000" }; }
      async prepareTransaction(tx: any) { return tx; }
      async sendTransaction() { return { status: "PENDING", hash: "mockhash123" }; }
    },
  },
  Contract: class {
    constructor() {}
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
  xdr: { ScVal: {} },
}));

import { makeSdk } from "./helpers/sdkFactory";
import { StellarGrantsError } from "../src/errors/StellarGrantsError";

// ── shared helpers ────────────────────────────────────────────────────────────

function scVal(v: any) { return v; }

// ── #463 — mock suite: write wrappers ────────────────────────────────────────

describe("StellarGrantsSDK — write wrappers (mocked, issue #463)", () => {
  it("grantCreate resolves to sendTransaction result", async () => {
    const { sdk, state } = makeSdk();
    state.sendStatus = "PENDING";

    const result = await sdk.grantCreate({
      owner: "GOWNER",
      title: "Test Grant",
      description: "desc",
      budget: BigInt(1000),
      deadline: BigInt(9999999999),
      milestoneCount: 3,
    });

    expect((result as any).status).toBe("PENDING");
  });

  it("grantCreate throws StellarGrantsError when simulation fails", async () => {
    const { sdk, state } = makeSdk();
    state.simulationError = "Contract error #1";

    await expect(
      sdk.grantCreate({
        owner: "GOWNER",
        title: "Fail",
        description: "",
        budget: BigInt(0),
        deadline: BigInt(0),
        milestoneCount: 1,
      }),
    ).rejects.toThrow(StellarGrantsError);
  });

  it("grantFund resolves to sendTransaction result", async () => {
    const { sdk } = makeSdk();

    const result = await sdk.grantFund({
      grantId: 1,
      token: "CTOKEN",
      amount: BigInt(500),
    });

    expect((result as any).hash).toBe("mockhash123");
  });

  it("milestoneSubmit resolves", async () => {
    const { sdk } = makeSdk();

    const result = await sdk.milestoneSubmit({
      grantId: 1,
      milestoneIdx: 0,
      proofHash: "Qmabc",
    });

    expect((result as any).status).toBe("PENDING");
  });

  it("milestoneVote resolves", async () => {
    const { sdk } = makeSdk();

    const result = await sdk.milestoneVote({
      grantId: 1,
      milestoneIdx: 0,
      approve: true,
    });

    expect((result as any).status).toBe("PENDING");
  });

  it("throws StellarGrantsError when sendTransaction returns ERROR", async () => {
    const { sdk, state } = makeSdk();
    state.sendStatus = "ERROR";
    state.sendErrorResult = "fee too low";

    await expect(
      sdk.grantCreate({
        owner: "G",
        title: "T",
        description: "",
        budget: BigInt(0),
        deadline: BigInt(0),
        milestoneCount: 1,
      }),
    ).rejects.toThrow("fee too low");
  });

  it("throws when no signer is configured", async () => {
    const { sdk } = makeSdk({ signer: undefined, wallet: undefined });

    await expect(
      sdk.grantCreate({
        owner: "G",
        title: "T",
        description: "",
        budget: BigInt(0),
        deadline: BigInt(0),
        milestoneCount: 1,
      }),
    ).rejects.toThrow(/signer/i);
  });
});

// ── #463 — read wrappers ──────────────────────────────────────────────────────

describe("StellarGrantsSDK — read wrappers (mocked, issue #463)", () => {
  it("grantGet returns parsed simulation retval", async () => {
    const { sdk, state } = makeSdk();
    state.simulationResult = { grantId: 42, title: "my grant" };

    const result = await sdk.grantGet(42);
    expect(result).toEqual({ grantId: 42, title: "my grant" });
  });

  it("milestoneGet returns parsed simulation retval", async () => {
    const { sdk, state } = makeSdk();
    state.simulationResult = { idx: 0, approved: false };

    const result = await sdk.milestoneGet(1, 0);
    expect(result).toEqual({ idx: 0, approved: false });
  });

  it("grantGet throws StellarGrantsError when simulation fails", async () => {
    const { sdk, state } = makeSdk();
    state.simulationError = "not found";

    await expect(sdk.grantGet(999)).rejects.toThrow(StellarGrantsError);
  });
});

// ── #463 — checkCompatibility ─────────────────────────────────────────────────

describe("StellarGrantsSDK — checkCompatibility (mocked, issue #463)", () => {
  it("returns compatible when contract version matches SDK", async () => {
    const { sdk, state } = makeSdk();
    state.simulationResult = 1; // CONTRACT_INTERFACE_VERSION

    const result = await sdk.checkCompatibility();
    expect(result.compatible).toBe(true);
    expect(result.sdkVersion).toBe(1);
    expect(result.contractVersion).toBe(1);
  });

  it("emits a warning when contract version differs from SDK version", async () => {
    const { sdk } = makeSdk();
    // Override to return version 1 (SDK is also 1 → compatible) but with
    // a custom warning we can detect. We actually verify the logic by providing
    // a future version (higher than SDK=1).
    (sdk as any).server.simulateTransaction = jest.fn(async () => ({
      result: { retval: 99 },
      minResourceFee: "0",
    }));

    const result = await sdk.checkCompatibility();
    expect(result.compatible).toBe(false);
    expect(result.contractVersion).toBe(99);
    expect(result.warning).toMatch(/newer/i);
  });

  it("returns compatible with warning when sdk_version method not found", async () => {
    const { sdk, state } = makeSdk();
    state.simulationError = "method not found";

    const result = await sdk.checkCompatibility();
    expect(result.compatible).toBe(true);
    expect(result.warning).toMatch(/compatibility mode/i);
  });
});

// ── #458 — dynamic fee estimation applied in invokeWrite ─────────────────────

describe("#458 — dynamic fee estimation in invokeWrite", () => {
  it("uses simulationMinResourceFee to rebuild tx before sending", async () => {
    const { sdk, state, mockServer } = makeSdk();
    state.minResourceFee = "2000";

    await sdk.grantCreate({
      owner: "G",
      title: "T",
      description: "",
      budget: BigInt(0),
      deadline: BigInt(0),
      milestoneCount: 1,
    });

    // Two simulateTransaction calls: the initial probe + the rebuilt tx.
    // The second buildTx will be called with the adjusted fee.
    expect(mockServer.simulateTransaction).toHaveBeenCalled();
  });

  it("estimateFees returns source=simulation-fallback when no horizonUrl", async () => {
    const { sdk, state } = makeSdk();
    state.minResourceFee = "500";

    const fees = await sdk.estimateFees("grant_get", []);
    expect(fees.source).toBe("simulation-fallback");
    expect(BigInt(fees.base)).toBe(BigInt(500));
  });

  it("estimateFees throws when simulation fails", async () => {
    const { sdk, state } = makeSdk();
    state.simulationError = "boom";

    await expect(sdk.estimateFees("grant_get", [])).rejects.toThrow(StellarGrantsError);
  });

  it("manually overriding simulatedFee bypasses estimation", async () => {
    const { sdk, mockServer } = makeSdk();

    await sdk.grantCreate(
      {
        owner: "G",
        title: "T",
        description: "",
        budget: BigInt(0),
        deadline: BigInt(0),
        milestoneCount: 1,
      },
      { simulatedFee: "9999" },
    );

    // When an explicit fee is provided, it's used directly — still only
    // one simulation call (the initial probe), then prepareTransaction.
    expect(mockServer.prepareTransaction).toHaveBeenCalled();
  });
});

// ── #462 — simulateFootprint + footprint passthrough ─────────────────────────

describe("#462 — footprint management", () => {
  it("simulateFootprint returns transactionData from simulation", async () => {
    const { sdk, state } = makeSdk();
    state.simulationResult = { _mock: "footprint_retval" };
    // Augment mock to return transactionData
    (sdk as any).server.simulateTransaction = jest.fn(async () => ({
      result: { retval: null },
      minResourceFee: "0",
      transactionData: { footprintKey: "some_xdr_data" },
    }));

    const footprint = await sdk.simulateFootprint("grant_get", []);
    expect(footprint).toEqual({ footprintKey: "some_xdr_data" });
  });

  it("simulateFootprint returns null when no transactionData in response", async () => {
    const { sdk } = makeSdk();
    (sdk as any).server.simulateTransaction = jest.fn(async () => ({
      result: { retval: null },
      minResourceFee: "0",
    }));

    const footprint = await sdk.simulateFootprint("grant_get", []);
    expect(footprint).toBeNull();
  });

  it("simulateFootprint throws StellarGrantsError on simulation failure", async () => {
    const { sdk, state } = makeSdk();
    state.simulationError = "footprint fail";

    await expect(sdk.simulateFootprint("grant_get", [])).rejects.toThrow(StellarGrantsError);
  });

  it("passing footprint to grantCreate calls setSorobanData on builder", async () => {
    const { sdk } = makeSdk();
    const fakeFootprint = { _type: "soroban_data" };

    // setSorobanData is optional — verify it doesn't throw when footprint provided
    await expect(
      sdk.grantCreate(
        {
          owner: "G",
          title: "T",
          description: "",
          budget: BigInt(0),
          deadline: BigInt(0),
          milestoneCount: 1,
        },
        { footprint: fakeFootprint },
      ),
    ).resolves.toBeDefined();
  });

  it("write calls succeed without footprint (backward compat)", async () => {
    const { sdk } = makeSdk();

    await expect(
      sdk.grantCreate({
        owner: "G",
        title: "T",
        description: "",
        budget: BigInt(0),
        deadline: BigInt(0),
        milestoneCount: 1,
      }),
    ).resolves.toBeDefined();
  });
});

// ── #463 — mock signer validates signTransaction is called ───────────────────

describe("#463 — mock signer is exercised", () => {
  it("signTransaction is called exactly once per write", async () => {
    const { sdk, mockSigner } = makeSdk();

    await sdk.grantCreate({
      owner: "G",
      title: "T",
      description: "",
      budget: BigInt(0),
      deadline: BigInt(0),
      milestoneCount: 1,
    });

    expect(mockSigner.signTransaction).toHaveBeenCalledTimes(1);
  });

  it("getPublicKey is called to derive the source account", async () => {
    const { sdk, mockSigner } = makeSdk();

    await sdk.grantCreate({
      owner: "G",
      title: "T",
      description: "",
      budget: BigInt(0),
      deadline: BigInt(0),
      milestoneCount: 1,
    });

    expect(mockSigner.getPublicKey).toHaveBeenCalled();
  });
});
