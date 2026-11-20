// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Account, Asset, BASE_FEE, Keypair, Networks, Operation, TransactionBuilder } from "@stellar/stellar-sdk";
import {
  getAddress,
  getNetworkDetails,
  isAllowed,
  setAllowed,
  signTransaction,
} from "@stellar/freighter-api";
import {
  FreighterAdapter,
  FreighterWalletError,
  classifyFreighterMessage,
  toFreighterWalletError,
} from "@/lib/wallets/FreighterAdapter";

vi.mock("@stellar/freighter-api", () => ({
  getAddress: vi.fn(),
  getNetworkDetails: vi.fn(),
  isAllowed: vi.fn(),
  setAllowed: vi.fn(),
  signTransaction: vi.fn(),
}));

const ADDRESS = "GCK4LZF4RXGPTVCQFSAZ5RYS7CY26BSSML5TJLSBVWUNFJDTNGJ2LOWH";

const freighterWindow = window as Window & { freighter?: unknown };

function installFreighter(): void {
  freighterWindow.freighter = {};
}

async function expectFreighterError(
  promise: Promise<unknown>,
): Promise<FreighterWalletError> {
  const error = await promise.then(
    () => {
      throw new Error("expected the call to reject");
    },
    (reason: unknown) => reason,
  );
  expect(error).toBeInstanceOf(FreighterWalletError);
  return error as FreighterWalletError;
}

function buildTransactionXdr(): string {
  const source = Keypair.random().publicKey();
  const destination = Keypair.random().publicKey();
  return new TransactionBuilder(new Account(source, "0"), {
    fee: BASE_FEE.toString(),
    networkPassphrase: Networks.TESTNET,
  })
    .addOperation(
      Operation.payment({
        destination,
        asset: Asset.native(),
        amount: "10",
      }),
    )
    .setTimeout(30)
    .build()
    .toXDR();
}

beforeEach(() => {
  vi.clearAllMocks();
  delete freighterWindow.freighter;
});

describe("FreighterAdapter error states", () => {
  it("reports availability synchronously", () => {
    const adapter = new FreighterAdapter({ injectionTimeoutMs: 0 });
    expect(adapter.isAvailable()).toBe(false);
    installFreighter();
    expect(adapter.isAvailable()).toBe(true);
  });

  it("raises NOT_INSTALLED with install guidance when Freighter never injects", async () => {
    const adapter = new FreighterAdapter({ injectionTimeoutMs: 0 });

    const error = await expectFreighterError(adapter.getPublicKey());

    expect(error.code).toBe("NOT_INSTALLED");
    expect(error.message).toMatch(/install or enable the Freighter extension/i);
    expect(isAllowed).not.toHaveBeenCalled();
  });

  it("waits for the extension to inject window.freighter", async () => {
    vi.mocked(isAllowed).mockResolvedValue({ isAllowed: true });
    vi.mocked(getAddress).mockResolvedValue({ address: ADDRESS });
    setTimeout(installFreighter, 150);

    const adapter = new FreighterAdapter({ injectionTimeoutMs: 2000 });

    await expect(adapter.getPublicKey()).resolves.toBe(ADDRESS);
  });

  it("raises LOCKED when the wallet is locked", async () => {
    installFreighter();
    vi.mocked(isAllowed).mockResolvedValue({ isAllowed: true });
    vi.mocked(getAddress).mockResolvedValue({
      address: "",
      error: { code: 403, message: "Freighter is locked" },
    });

    const error = await expectFreighterError(new FreighterAdapter().getPublicKey());

    expect(error.code).toBe("LOCKED");
    expect(error.message).toMatch(/unlock it, and try again/i);
  });

  it("raises REJECTED when the connection request is not approved", async () => {
    installFreighter();
    vi.mocked(isAllowed).mockResolvedValue({ isAllowed: false });
    vi.mocked(setAllowed).mockResolvedValue({ isAllowed: false });

    const error = await expectFreighterError(new FreighterAdapter().getPublicKey());

    expect(error.code).toBe("REJECTED");
    expect(error.message).toMatch(/connection request was not approved/i);
    expect(setAllowed).toHaveBeenCalled();
  });

  it("raises REJECTED when the user rejects a signing request", async () => {
    installFreighter();
    vi.mocked(getNetworkDetails).mockResolvedValue({
      network: "TESTNET",
      networkUrl: "https://horizon-testnet.stellar.org",
      networkPassphrase: Networks.TESTNET,
    });
    vi.mocked(signTransaction).mockResolvedValue({
      signedTxXdr: "",
      signerAddress: ADDRESS,
      error: { code: 4001, message: "User rejected the request" },
    });

    const error = await expectFreighterError(
      new FreighterAdapter().signTransaction(buildTransactionXdr(), Networks.TESTNET),
    );

    expect(error.code).toBe("REJECTED");
    expect(error.message).toMatch(/rejected in Freighter/i);
    expect(error.message).not.toContain("[object Object]");
  });

  it("raises NETWORK_MISMATCH naming both networks", async () => {
    installFreighter();
    vi.mocked(getNetworkDetails).mockResolvedValue({
      network: "TESTNET",
      networkUrl: "https://horizon-testnet.stellar.org",
      networkPassphrase: Networks.TESTNET,
    });

    const error = await expectFreighterError(
      new FreighterAdapter().signTransaction(buildTransactionXdr(), Networks.PUBLIC),
    );

    expect(error.code).toBe("NETWORK_MISMATCH");
    expect(error.message).toMatch(/wrong network \(TESTNET\)/i);
    expect(error.message).toMatch(/switch it to Public/i);
    expect(signTransaction).not.toHaveBeenCalled();
  });

  it("keeps the underlying detail for unknown failures", () => {
    const error = toFreighterWalletError(new Error("something unexpected exploded"));

    expect(error.code).toBe("UNKNOWN");
    expect(error.message).toContain("something unexpected exploded");
    expect(error.message).not.toContain("[object Object]");
  });

  it("returns the same instance when already normalised", () => {
    const original = new FreighterWalletError("LOCKED", "locked");
    expect(toFreighterWalletError(original)).toBe(original);
  });

  it("signs on the matching network and returns the signed XDR", async () => {
    installFreighter();
    const txXdr = buildTransactionXdr();
    vi.mocked(getNetworkDetails).mockResolvedValue({
      network: "TESTNET",
      networkUrl: "https://horizon-testnet.stellar.org",
      networkPassphrase: Networks.TESTNET,
    });
    vi.mocked(signTransaction).mockResolvedValue({
      signedTxXdr: txXdr,
      signerAddress: ADDRESS,
    });

    const signed = await new FreighterAdapter().signTransaction(txXdr, Networks.TESTNET);

    expect(signed).toBe(txXdr);
    expect(signTransaction).toHaveBeenCalledWith(txXdr, {
      networkPassphrase: Networks.TESTNET,
    });
  });
});

describe("classifyFreighterMessage", () => {
  it.each([
    ["Freighter is not installed", "NOT_INSTALLED"],
    ["Extension context invalidated", "NOT_INSTALLED"],
    ["Please unlock Freighter", "LOCKED"],
    ["wallet locked", "LOCKED"],
    ["User declined the transaction", "REJECTED"],
    ["Request timed out waiting for approval", "REJECTED"],
    ["Account sequence mismatch", "UNKNOWN"],
  ])("classifies %j as %s", (message, expected) => {
    expect(classifyFreighterMessage(message)).toBe(expected);
  });

  it("never renders object errors as [object Object]", () => {
    const error = toFreighterWalletError({ code: 500, nested: true });

    expect(error.message).not.toContain("[object Object]");
    expect(error.code).toBe("UNKNOWN");
  });
});
