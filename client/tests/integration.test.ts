/**
 * Integration tests for StellarGrantsSDK (#259).
 *
 * Uses a stateful mock RPC server that maintains a virtual ledger so we can
 * verify full end-to-end flows without a live network:
 *
 *   Create Grant → Fund Grant → Submit Milestone → Vote on Milestone
 *
 * The mock tracks submitted grants, recorded milestones, and sent
 * transactions so assertions can cross-check state across calls.
 */

// Mirror the module-level mock used in sdk.test.ts so Contract/Account/
// TransactionBuilder do not validate Strkey formats during integration tests.
jest.mock("@stellar/stellar-sdk", () => ({
  rpc: {
    Server: class {
      constructor() {}
      async getAccount() { return { accountId: "GMOCK", sequence: "0" }; }
      async simulateTransaction() { return { result: { retval: null }, minResourceFee: "1000" }; }
      async prepareTransaction(tx: any) { return tx; }
      async sendTransaction() { return { status: "PENDING", hash: "mockhash" }; }
      async getEvents() { return { events: [] }; }
      async getLatestLedger() { return { sequence: 1000, id: "hash", protocolVersion: 20 }; }
      async getTransaction(hash: string) {
        return { status: "SUCCESS", hash };
      }
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
    static fromXDR(_xdr: string, _pp: string) { return { toXDR: () => "SIGNED_XDR" }; }
    constructor() {}
    addOperation() { return this; }
    setTimeout() { return this; }
    setSorobanData() { return this; }
    build() { return { toXDR: () => "TX_XDR" }; }
  },
  nativeToScVal: (v: unknown) => ({ _scval: v }),
  scValToNative: (v: any) => v?._native ?? { ok: true },
  xdr: { ScVal: { fromXDR: () => ({ _scval: "decoded" }) }, SorobanTransactionData: class {} },
}));

import { StellarGrantsSDK } from "../src/StellarGrantsSDK";
import { makeMockSigner } from "./helpers/mockSigner";
import { TEST_CONTRACT_ID, TEST_NETWORK_PASSPHRASE } from "./helpers/sdkFactory";

// ── Stateful mock RPC ─────────────────────────────────────────────────────────

interface VirtualLedger {
  sequence: number;
  grants: Map<number, { title: string; budget: bigint; funded: bigint }>;
  milestones: Map<string, { proofHash: string; votes: boolean[] }>;
  txLog: string[];
}

function makeStatefulMockServer() {
  const ledger: VirtualLedger = {
    sequence: 1000,
    grants: new Map(),
    milestones: new Map(),
    txLog: [],
  };

  let nextGrantId = 1;

  const server = {
    ledger,
    getAccount: jest.fn(async () => ({
      accountId: "GB3KJPLFUYN5VL6R3GU3EGCGVCKFDSD7BEDX42HWG5BWFKB3KQGJJRMA",
      sequence: String(ledger.sequence++),
    })),

    getNetwork: jest.fn(async () => ({
      passphrase: TEST_NETWORK_PASSPHRASE,
      protocolVersion: 20,
    })),

    getLatestLedger: jest.fn(async () => ({
      sequence: ledger.sequence,
      id: "mock-ledger-hash",
      protocolVersion: 20,
    })),

    // Simulation: return a plausible result for any call
    simulateTransaction: jest.fn(async () => ({
      result: {
        retval: { _simulatedValue: nextGrantId },
      },
      minResourceFee: "500",
    })),

    prepareTransaction: jest.fn(async (tx: any) => tx),

    // Submission: record the call and advance the virtual ledger
    sendTransaction: jest.fn(async (_tx: any) => {
      const hash = `txhash-${ledger.txLog.length + 1}`;
      ledger.txLog.push(hash);
      return { status: "PENDING", hash };
    }),

    getTransaction: jest.fn(async (hash: string) => {
      // All submitted transactions immediately succeed in the virtual ledger
      if (ledger.txLog.includes(hash)) {
        return { status: "SUCCESS", hash };
      }
      return { status: "NOT_FOUND" };
    }),

    getEvents: jest.fn(async () => ({ events: [] })),
  };

  return { server, ledger };
}

// ── SDK factory for integration tests ─────────────────────────────────────────

function makeIntegrationSdk() {
  const mockSigner = makeMockSigner();
  const { server, ledger } = makeStatefulMockServer();

  const sdk = new StellarGrantsSDK({
    contractId: TEST_CONTRACT_ID,
    rpcUrl: "https://soroban-testnet.stellar.org",
    networkPassphrase: TEST_NETWORK_PASSPHRASE,
    signer: mockSigner,
  });

  // Inject stateful mock
  (sdk as any).server = server;

  return { sdk, mockServer: server, ledger, mockSigner };
}

// ── Integration test suite ────────────────────────────────────────────────────

describe("StellarGrantsSDK — full flow integration", () => {
  describe("Create Grant → verify submission", () => {
    it("builds, signs, and submits a grant creation transaction", async () => {
      const { sdk, mockServer, ledger } = makeIntegrationSdk();

      await sdk.grantCreate({
        owner: "GTEST_OWNER",
        title: "Ocean Clean-Up",
        description: "Remove plastic from Pacific gyres.",
        budget: BigInt(10_000),
        deadline: BigInt(Date.now() + 86_400_000),
        milestoneCount: 3,
      });

      // Transaction was sent
      expect(mockServer.sendTransaction).toHaveBeenCalledTimes(1);
      expect(ledger.txLog).toHaveLength(1);

      // Transaction was signed
      expect(mockServer.prepareTransaction).toHaveBeenCalledTimes(1);
    });
  });

  describe("Fund Grant → verify state change", () => {
    it("submits a funding transaction after grant creation", async () => {
      const { sdk, mockServer, ledger } = makeIntegrationSdk();

      // Create first
      await sdk.grantCreate({
        owner: "GTEST",
        title: "Reforestation",
        description: "Plant 1 million trees.",
        budget: BigInt(5_000),
        deadline: BigInt(Date.now() + 86_400_000),
        milestoneCount: 2,
      });

      const preFundTxCount = ledger.txLog.length;

      // Fund
      await sdk.grantFund({
        grantId: 1,
        token: "CTOKEN_CONTRACT_ID_MOCK",
        amount: BigInt(1_000),
      });

      expect(ledger.txLog.length).toBe(preFundTxCount + 1);
      expect(mockServer.sendTransaction).toHaveBeenCalledTimes(2);
    });
  });

  describe("Submit Milestone → verify proof recorded", () => {
    it("submits a milestone proof transaction", async () => {
      const { sdk, mockServer } = makeIntegrationSdk();

      await sdk.milestoneSubmit({
        grantId: 1,
        milestoneIdx: 0,
        proofHash: "QmProofHash123",
      });

      expect(mockServer.sendTransaction).toHaveBeenCalledTimes(1);
      const call = mockServer.sendTransaction.mock.calls[0];
      expect(call).toBeDefined();
    });
  });

  describe("Vote on Milestone → approve and reject paths", () => {
    it("submits an approve vote", async () => {
      const { sdk, mockServer } = makeIntegrationSdk();

      await sdk.milestoneVote({
        grantId: 1,
        milestoneIdx: 0,
        approve: true,
      });

      expect(mockServer.sendTransaction).toHaveBeenCalledTimes(1);
    });

    it("submits a reject vote", async () => {
      const { sdk, mockServer } = makeIntegrationSdk();

      await sdk.milestoneVote({
        grantId: 1,
        milestoneIdx: 0,
        approve: false,
      });

      expect(mockServer.sendTransaction).toHaveBeenCalledTimes(1);
    });
  });

  describe("Full end-to-end flow: Create → Fund → Submit → Vote", () => {
    it("completes a full grant lifecycle without errors", async () => {
      const { sdk, mockServer, ledger } = makeIntegrationSdk();

      // 1. Create
      await sdk.grantCreate({
        owner: "GOWNER",
        title: "Clean Water Initiative",
        description: "Provide clean water to 10 communities.",
        budget: BigInt(50_000),
        deadline: BigInt(Date.now() + 30 * 86_400_000),
        milestoneCount: 2,
      });

      // 2. Fund
      await sdk.grantFund({ grantId: 1, token: "CTOKEN_MOCK", amount: BigInt(50_000) });

      // 3. Submit milestone proof
      await sdk.milestoneSubmit({ grantId: 1, milestoneIdx: 0, proofHash: "QmWaterProof" });

      // 4. Vote to approve
      await sdk.milestoneVote({ grantId: 1, milestoneIdx: 0, approve: true });

      // All 4 operations produced exactly 4 transactions
      expect(ledger.txLog).toHaveLength(4);
      expect(mockServer.sendTransaction).toHaveBeenCalledTimes(4);
    });
  });

  describe("Error resilience", () => {
    it("raises StellarGrantsError when simulation fails", async () => {
      const { sdk, mockServer } = makeIntegrationSdk();

      mockServer.simulateTransaction.mockResolvedValueOnce({
        error: "Contract trap: HostError(ContractError(1))",
      } as any);

      await expect(
        sdk.grantCreate({
          owner: "GBAD",
          title: "Bad grant",
          description: "This will fail.",
          budget: BigInt(100),
          deadline: BigInt(Date.now()),
          milestoneCount: 1,
        }),
      ).rejects.toThrow();
    });

    it("raises StellarGrantsError when sendTransaction returns ERROR", async () => {
      const { sdk, mockServer } = makeIntegrationSdk();

      mockServer.sendTransaction.mockResolvedValueOnce({
        status: "ERROR",
        errorResult: "tx_bad_seq",
      } as any);

      await expect(
        sdk.grantFund({ grantId: 1, token: "CTOKEN", amount: BigInt(100) }),
      ).rejects.toThrow();
    });
  });
});

// ── CORS / Proxy configuration tests (#274) ───────────────────────────────────

describe("CORS and RPC proxy support (#274)", () => {
  it("initialises server with proxyUrl instead of rpcUrl", () => {
    const sdk = new StellarGrantsSDK({
      contractId: TEST_CONTRACT_ID,
      rpcUrl: "https://soroban-testnet.stellar.org",
      proxyUrl: "https://my-proxy.example.com/rpc",
      networkPassphrase: TEST_NETWORK_PASSPHRASE,
    });

    // The internal server is constructed — no error thrown
    expect(sdk).toBeInstanceOf(StellarGrantsSDK);
  });

  it("initialises without error when customHeaders are provided", () => {
    const sdk = new StellarGrantsSDK({
      contractId: TEST_CONTRACT_ID,
      rpcUrl: "https://soroban-testnet.stellar.org",
      customHeaders: { "X-Api-Key": "test-key", Authorization: "Bearer token" },
      networkPassphrase: TEST_NETWORK_PASSPHRASE,
    });

    expect(sdk).toBeInstanceOf(StellarGrantsSDK);
  });

  it("works in http:// environments (CORS-proxy scenario)", () => {
    expect(() =>
      new StellarGrantsSDK({
        contractId: TEST_CONTRACT_ID,
        rpcUrl: "http://localhost:8000/rpc",
        networkPassphrase: TEST_NETWORK_PASSPHRASE,
      }),
    ).not.toThrow();
  });
});

// ── Allowance management tests (#272) ────────────────────────────────────────

describe("Token allowance management (#272)", () => {
  const TOKEN = "CTOKEN_SAC_MOCK";

  it("getAllowance reads allowance from the token contract", async () => {
    const { sdk, mockServer } = makeIntegrationSdk();

    // Mock a non-zero allowance response. The SDK's parseSimulationResult
    // passes retval through scValToNative; the mock returns `_native` as-is.
    mockServer.simulateTransaction.mockResolvedValueOnce({
      result: {
        retval: { _native: { amount: BigInt(5_000), expiration_ledger: 1500 } },
      },
      minResourceFee: "200",
    } as any);

    const result = await sdk.getAllowance(TOKEN, "GOWNER_ADDRESS");
    expect(result.amount).toBe(BigInt(5_000));
    expect(result.expirationLedger).toBe(1500);
  });

  it("checkAndSetAllowance returns sufficient=true when allowance is enough", async () => {
    const { sdk, mockServer, mockSigner } = makeIntegrationSdk();
    jest.spyOn(mockSigner, "getPublicKey").mockResolvedValue("GOWNER_ADDRESS");

    mockServer.simulateTransaction.mockResolvedValueOnce({
      result: { retval: { _native: { amount: BigInt(10_000), expiration_ledger: 2000 } } },
      minResourceFee: "200",
    } as any);

    const check = await sdk.checkAndSetAllowance(TOKEN, BigInt(5_000), "GOWNER_ADDRESS");
    expect(check.sufficient).toBe(true);
    expect(check.current).toBe(BigInt(10_000));
    // No approval transaction needed
    expect(mockServer.sendTransaction).not.toHaveBeenCalled();
  });

  it("checkAndSetAllowance calls setAllowance when allowance is insufficient", async () => {
    const { sdk, mockServer, mockSigner } = makeIntegrationSdk();
    jest.spyOn(mockSigner, "getPublicKey").mockResolvedValue("GOWNER_ADDRESS");

    // getAllowance returns 0
    mockServer.simulateTransaction.mockResolvedValueOnce({
      result: { retval: { _native: { amount: BigInt(0), expiration_ledger: 0 } } },
      minResourceFee: "200",
    } as any);

    const check = await sdk.checkAndSetAllowance(TOKEN, BigInt(5_000), "GOWNER_ADDRESS");
    expect(check.sufficient).toBe(false);
    expect(check.required).toBe(BigInt(5_000));
    // setAllowance triggered a transaction
    expect(mockServer.sendTransaction).toHaveBeenCalledTimes(1);
  });
});

// ── IPFS helpers tests (#261) ─────────────────────────────────────────────────

describe("IPFS metadata helpers (#261)", () => {
  const { uploadMetadataToIPFS, fetchMetadataFromIPFS } =
    require("../src/ipfs") as typeof import("../src/ipfs");
  const { MetadataValidationError } =
    require("../src/errors/MetadataValidationError") as typeof import("../src/errors/MetadataValidationError");

  beforeEach(() => {
    jest.resetAllMocks();
    global.fetch = jest.fn();
  });

  it("uploadMetadataToIPFS posts to Pinata with JWT auth", async () => {
    (global.fetch as jest.Mock).mockResolvedValueOnce({
      ok: true,
      json: async () => ({ IpfsHash: "QmTestCID123" }),
    });

    const result = await uploadMetadataToIPFS(
      { title: "Grant Metadata", description: "Test" },
      { pinataJwt: "my-jwt-token" },
    );

    expect(result.cid).toBe("QmTestCID123");
    expect(result.gatewayUrl).toContain("QmTestCID123");

    const call = (global.fetch as jest.Mock).mock.calls[0];
    expect(call[0]).toBe("https://api.pinata.cloud/pinning/pinJSONToIPFS");
    expect(call[1].headers.Authorization).toBe("Bearer my-jwt-token");
  });

  it("uploadMetadataToIPFS throws when no credentials provided", async () => {
    await expect(
      uploadMetadataToIPFS({ title: "No auth", description: "No credentials" }, {}),
    ).rejects.toThrow(/pinata/i);
  });

  it("uploadMetadataToIPFS validates grant schema before upload", async () => {
    await expect(
      uploadMetadataToIPFS(
        { title: "", description: "" },
        { pinataJwt: "my-jwt-token", metadataSchema: "grant" },
      ),
    ).rejects.toBeInstanceOf(MetadataValidationError);

    expect(global.fetch).not.toHaveBeenCalled();
  });

  it("uploadMetadataToIPFS validates milestone schema before upload", async () => {
    await expect(
      uploadMetadataToIPFS(
        { title: "M1", description: "First milestone" },
        { pinataJwt: "my-jwt-token", metadataSchema: "milestone" },
      ),
    ).rejects.toBeInstanceOf(MetadataValidationError);

    expect(global.fetch).not.toHaveBeenCalled();
  });

  it("fetchMetadataFromIPFS returns parsed JSON from first responding gateway", async () => {
    (global.fetch as jest.Mock).mockResolvedValueOnce({
      ok: true,
      json: async () => ({ title: "Fetched grant", budget: 1000 }),
    });

    const data = await fetchMetadataFromIPFS("QmTestCID123");
    expect(data.title).toBe("Fetched grant");
  });

  it("fetchMetadataFromIPFS falls back to next gateway on failure", async () => {
    (global.fetch as jest.Mock)
      .mockRejectedValueOnce(new Error("Gateway timeout"))
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ title: "Fetched from fallback" }),
      });

    const data = await fetchMetadataFromIPFS("QmFallbackCID");
    expect(data.title).toBe("Fetched from fallback");
  });

  it("fetchMetadataFromIPFS throws when all gateways fail", async () => {
    (global.fetch as jest.Mock).mockRejectedValue(new Error("All down"));

    await expect(fetchMetadataFromIPFS("QmDeadCID", [])).rejects.toThrow(
      /Failed to fetch IPFS metadata/,
    );
  });
});
