/**
 * FreighterAdapter tests
 *
 * The adapter now uses named imports from @stellar/freighter-api (v2 API).
 * All Freighter functions are mocked at the module level.
 */

import { FreighterAdapter } from "../../src/wallets/FreighterAdapter";

const TESTNET_PASSPHRASE = "Test SDF Network ; September 2015";
const MAINNET_PASSPHRASE = "Public Global Stellar Network ; September 2015";

// ── Module-level mock for @stellar/freighter-api ──────────────────────────────

jest.mock("@stellar/freighter-api", () => ({
  isAllowed: jest.fn(),
  setAllowed: jest.fn(),
  getAddress: jest.fn(),
  getNetworkDetails: jest.fn(),
  signTransaction: jest.fn(),
}));

import {
  isAllowed,
  setAllowed,
  getAddress,
  getNetworkDetails,
  signTransaction as freighterSignTransaction,
} from "@stellar/freighter-api";

const mockIsAllowed = isAllowed as jest.Mock;
const mockSetAllowed = setAllowed as jest.Mock;
const mockGetAddress = getAddress as jest.Mock;
const mockGetNetworkDetails = getNetworkDetails as jest.Mock;
const mockSignTransaction = freighterSignTransaction as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
  // Default: already allowed
  mockIsAllowed.mockResolvedValue({ isAllowed: true });
  mockSetAllowed.mockResolvedValue({ isAllowed: true });
  mockGetAddress.mockResolvedValue({ address: "GFREIGHTER_KEY", error: undefined });
  mockGetNetworkDetails.mockResolvedValue({ networkPassphrase: TESTNET_PASSPHRASE, network: "TESTNET" });
  mockSignTransaction.mockResolvedValue({ signedTxXdr: "FREIGHTER_SIGNED_XDR", error: undefined });
});

// ── isAvailable ───────────────────────────────────────────────────────────────

describe("FreighterAdapter.isAvailable", () => {
  afterEach(() => {
    delete (global as any).window;
  });

  it("returns true when window.freighter is present", () => {
    (global as any).window = { freighter: {} };
    expect(new FreighterAdapter().isAvailable()).toBe(true);
  });

  it("returns false when window.freighter is absent", () => {
    (global as any).window = {};
    expect(new FreighterAdapter().isAvailable()).toBe(false);
  });

  it("returns false in non-browser environments", () => {
    delete (global as any).window;
    expect(new FreighterAdapter().isAvailable()).toBe(false);
  });
});

// ── getPublicKey ──────────────────────────────────────────────────────────────

describe("FreighterAdapter.getPublicKey", () => {
  it("returns public key when already allowed", async () => {
    const adapter = new FreighterAdapter();
    const key = await adapter.getPublicKey();

    expect(mockIsAllowed).toHaveBeenCalledTimes(1);
    expect(mockSetAllowed).not.toHaveBeenCalled();
    expect(key).toBe("GFREIGHTER_KEY");
  });

  it("calls setAllowed when not yet allowed, then returns key", async () => {
    mockIsAllowed.mockResolvedValue({ isAllowed: false });
    mockSetAllowed.mockResolvedValue({ isAllowed: true });

    const adapter = new FreighterAdapter();
    const key = await adapter.getPublicKey();

    expect(mockSetAllowed).toHaveBeenCalledTimes(1);
    expect(key).toBe("GFREIGHTER_KEY");
  });

  it("throws when setAllowed is denied", async () => {
    mockIsAllowed.mockResolvedValue({ isAllowed: false });
    mockSetAllowed.mockResolvedValue({ isAllowed: false });

    const adapter = new FreighterAdapter();
    await expect(adapter.getPublicKey()).rejects.toThrow("Freighter permission denied");
  });

  it("throws when getAddress returns an error", async () => {
    mockGetAddress.mockResolvedValue({ address: undefined, error: "User rejected" });

    const adapter = new FreighterAdapter();
    await expect(adapter.getPublicKey()).rejects.toThrow("User rejected");
  });

  it("throws when getAddress returns no address", async () => {
    mockGetAddress.mockResolvedValue({ address: "", error: undefined });

    const adapter = new FreighterAdapter();
    await expect(adapter.getPublicKey()).rejects.toThrow("No public key returned by Freighter");
  });
});

// ── signTransaction ───────────────────────────────────────────────────────────

describe("FreighterAdapter.signTransaction", () => {
  it("signs and returns signedTxXdr on success", async () => {
    const adapter = new FreighterAdapter();
    const result = await adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE);

    expect(mockSignTransaction).toHaveBeenCalledWith("TX_XDR", {
      networkPassphrase: TESTNET_PASSPHRASE,
    });
    expect(result).toBe("FREIGHTER_SIGNED_XDR");
  });

  it("throws when Freighter is on the wrong network", async () => {
    mockGetNetworkDetails.mockResolvedValue({
      networkPassphrase: MAINNET_PASSPHRASE,
      network: "PUBLIC",
    });

    const adapter = new FreighterAdapter();
    await expect(adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE)).rejects.toThrow(
      TESTNET_PASSPHRASE,
    );
    expect(mockSignTransaction).not.toHaveBeenCalled();
  });

  it("throws when signTransaction returns an error", async () => {
    mockSignTransaction.mockResolvedValue({ signedTxXdr: undefined, error: "User declined" });

    const adapter = new FreighterAdapter();
    await expect(adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE)).rejects.toThrow(
      "User declined",
    );
  });

  it("throws when signTransaction returns no XDR", async () => {
    mockSignTransaction.mockResolvedValue({ signedTxXdr: "", error: undefined });

    const adapter = new FreighterAdapter();
    await expect(adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE)).rejects.toThrow(
      "Failed to sign transaction with Freighter",
    );
  });

  it("works with mainnet passphrase", async () => {
    mockGetNetworkDetails.mockResolvedValue({
      networkPassphrase: MAINNET_PASSPHRASE,
      network: "PUBLIC",
    });

    const adapter = new FreighterAdapter();
    const result = await adapter.signTransaction("TX_XDR", MAINNET_PASSPHRASE);

    expect(mockSignTransaction).toHaveBeenCalledWith("TX_XDR", {
      networkPassphrase: MAINNET_PASSPHRASE,
    });
    expect(result).toBe("FREIGHTER_SIGNED_XDR");
  });
});
