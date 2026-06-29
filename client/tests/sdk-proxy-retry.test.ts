/**
 * Tests for CORS/RPC proxy support (#495) and exponential back-off retry
 * integration (#502) in StellarGrantsSDK.
 */
import { StellarGrantsSDK } from "../src/StellarGrantsSDK";

jest.mock("@stellar/stellar-sdk", () => {
  class MockServer {
    constructor() {}
    async getAccount() {
      return { accountId: "GTEST", sequence: "1" };
    }
    async simulateTransaction() {
      return { result: { retval: { _mock: "ok" } } };
    }
    async prepareTransaction(tx: any) {
      return tx;
    }
    async sendTransaction() {
      return { status: "PENDING", hash: "abc123" };
    }
  }

  return {
    rpc: { Server: MockServer },
    Horizon: {
      Server: class {
        constructor() {}
      },
    },
    Contract: class {
      constructor() {}
      call(method: string, ...args: unknown[]) {
        return { method, args };
      }
    },
    TransactionBuilder: class {
      static fromXDR() {
        return { toXDR: () => "SIGNED_XDR", sign: () => {} };
      }
      constructor() {}
      addOperation() {
        return this;
      }
      setTimeout() {
        return this;
      }
      build() {
        return { toXDR: () => "TX_XDR" };
      }
    },
    nativeToScVal: (value: unknown) => ({ value }),
    scValToNative: () => ({ ok: true }),
    xdr: {},
  };
});

const TEST_CONTRACT = "CTEST_CONTRACT";
const TEST_PASSPHRASE = "Test SDF Network ; September 2015";

// ── CORS / Proxy (#495) ──────────────────────────────────────────────────────

describe("CORS and RPC proxy support (#495)", () => {
  it("constructs with rpcUrl alone", () => {
    expect(
      () =>
        new StellarGrantsSDK({
          contractId: TEST_CONTRACT,
          rpcUrl: "https://soroban-testnet.stellar.org",
          networkPassphrase: TEST_PASSPHRASE,
        }),
    ).not.toThrow();
  });

  it("constructs with proxyUrl instead of rpcUrl", () => {
    expect(
      () =>
        new StellarGrantsSDK({
          contractId: TEST_CONTRACT,
          proxyUrl: "https://my-proxy.example.com/rpc",
          networkPassphrase: TEST_PASSPHRASE,
        }),
    ).not.toThrow();
  });

  it("constructs with both proxyUrl and rpcUrl (proxy takes precedence)", () => {
    expect(
      () =>
        new StellarGrantsSDK({
          contractId: TEST_CONTRACT,
          rpcUrl: "https://soroban-testnet.stellar.org",
          proxyUrl: "https://proxy.corp.internal/stellar-rpc",
          networkPassphrase: TEST_PASSPHRASE,
        }),
    ).not.toThrow();
  });

  it("constructs with customHeaders for authenticated RPC", () => {
    expect(
      () =>
        new StellarGrantsSDK({
          contractId: TEST_CONTRACT,
          rpcUrl: "https://soroban-testnet.stellar.org",
          customHeaders: {
            Authorization: "Bearer my-jwt-token",
            "X-Api-Key": "enterprise-key",
          },
          networkPassphrase: TEST_PASSPHRASE,
        }),
    ).not.toThrow();
  });

  it("allows http:// endpoints for CORS-proxy scenarios", () => {
    expect(
      () =>
        new StellarGrantsSDK({
          contractId: TEST_CONTRACT,
          rpcUrl: "http://localhost:8000/rpc",
          networkPassphrase: TEST_PASSPHRASE,
        }),
    ).not.toThrow();
  });

  it("throws when neither rpcUrl nor proxyUrl is provided", () => {
    expect(
      () =>
        new StellarGrantsSDK({
          contractId: TEST_CONTRACT,
          networkPassphrase: TEST_PASSPHRASE,
        } as any),
    ).toThrow("Either rpcUrl or proxyUrl must be provided");
  });
});

// ── Retry config (#502) ──────────────────────────────────────────────────────

describe("Exponential backoff retry config (#502)", () => {
  it("constructs when retryConfig is provided", () => {
    expect(
      () =>
        new StellarGrantsSDK({
          contractId: TEST_CONTRACT,
          rpcUrl: "https://soroban-testnet.stellar.org",
          networkPassphrase: TEST_PASSPHRASE,
          retryConfig: { maxAttempts: 5, initialDelayMs: 500 },
        }),
    ).not.toThrow();
  });

  it("constructs without retryConfig (uses defaults)", () => {
    expect(
      () =>
        new StellarGrantsSDK({
          contractId: TEST_CONTRACT,
          rpcUrl: "https://soroban-testnet.stellar.org",
          networkPassphrase: TEST_PASSPHRASE,
        }),
    ).not.toThrow();
  });

  it("serverCall retries on 429 then succeeds", async () => {
    const sdk = new StellarGrantsSDK({
      contractId: TEST_CONTRACT,
      rpcUrl: "https://soroban-testnet.stellar.org",
      networkPassphrase: TEST_PASSPHRASE,
      retryConfig: { maxAttempts: 3, initialDelayMs: 0 },
    });

    let calls = 0;
    const fn = jest.fn().mockImplementation(() => {
      calls++;
      if (calls < 2) return Promise.reject(new Error("429 Too Many Requests"));
      return Promise.resolve("ok");
    });

    const result = await (sdk as any).serverCall(fn);
    expect(result).toBe("ok");
    expect(calls).toBe(2);
  });

  it("serverCall propagates hard failures immediately without retrying", async () => {
    const sdk = new StellarGrantsSDK({
      contractId: TEST_CONTRACT,
      rpcUrl: "https://soroban-testnet.stellar.org",
      networkPassphrase: TEST_PASSPHRASE,
      retryConfig: { maxAttempts: 3, initialDelayMs: 0 },
    });

    const fn = jest.fn().mockRejectedValue(new Error("Invalid signature"));
    await expect((sdk as any).serverCall(fn)).rejects.toThrow("Invalid signature");
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("serverCall exhausts retries and re-throws the last error", async () => {
    const sdk = new StellarGrantsSDK({
      contractId: TEST_CONTRACT,
      rpcUrl: "https://soroban-testnet.stellar.org",
      networkPassphrase: TEST_PASSPHRASE,
      retryConfig: { maxAttempts: 2, initialDelayMs: 0 },
    });

    const fn = jest.fn().mockRejectedValue(new Error("Server Busy"));
    await expect((sdk as any).serverCall(fn)).rejects.toThrow("Server Busy");
    expect(fn).toHaveBeenCalledTimes(2);
  });
});
