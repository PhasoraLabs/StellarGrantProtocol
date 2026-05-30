import { StellarGrantsSDK } from "../src/StellarGrantsSDK";
import { parseSorobanError } from "../src/errors/parseSorobanError";

jest.mock("@stellar/stellar-sdk", () => {
  class MockServer {
    static simulationError: string | null = null;
    constructor() {}
    async getAccount() {
      return { accountId: "GTEST", sequence: "1" };
    }
    async simulateTransaction() {
      if (MockServer.simulationError) {
        return { error: MockServer.simulationError };
      }
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
    Contract: class {
      constructor() {}
      call(method: string, ...args: unknown[]) {
        return { method, args };
      }
    },
    TransactionBuilder: class {
      static fromXDR() {
        return { from: "xdr" };
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

describe("StellarGrantsSDK", () => {
  const signer = {
    getPublicKey: jest.fn(async () => "GABC123"),
    signTransaction: jest.fn(async () => "SIGNED_XDR"),
  };

  it("calls write wrappers without stellar-cli dependency", async () => {
    const sdk = new StellarGrantsSDK({
      contractId: "CBLAH",
      rpcUrl: "https://rpc.test",
      networkPassphrase: "Test SDF Network ; September 2015",
      signer,
    });

    const result = await sdk.grantFund({
      grantId: 1,
      token: "GCTOKEN",
      amount: 1000n,
    });

    expect(result).toEqual({ status: "PENDING", hash: "abc123" });
    expect(signer.signTransaction).toHaveBeenCalled();
  });

  it("provides read wrapper response parsing", async () => {
    const sdk = new StellarGrantsSDK({
      contractId: "CBLAH",
      rpcUrl: "https://rpc.test",
      networkPassphrase: "Test SDF Network ; September 2015",
      signer,
    });

    const grant = await sdk.grantGet(7);
    expect(grant).toEqual({ ok: true });
  });

  it("parses generic Soroban revert errors", () => {
    const parsed = parseSorobanError(new Error("HostError: txFailed: revert: grant not active"));
    expect(parsed.name).toBe("SorobanRevertError");
    expect(parsed.message).toContain("grant not active");
  });
});
