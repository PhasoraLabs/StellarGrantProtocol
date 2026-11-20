import { AlbedoAdapter } from "../../src/wallets/AlbedoAdapter";
import { FreighterAdapter } from "../../src/wallets/FreighterAdapter";
import { XBullAdapter } from "../../src/wallets/XBullAdapter";
import { createPreferredWalletAdapter } from "../../src/wallets/createPreferredWalletAdapter";

// FreighterAdapter uses @stellar/freighter-api — mock it so isAvailable() works
// without the extension. isAvailable() only checks window.freighter, so no
// module mock is needed here; we just set the window global.

afterEach(() => {
  delete (global as any).window;
});

describe("createPreferredWalletAdapter", () => {
  it("returns FreighterAdapter when Freighter is available", () => {
    // FreighterAdapter.isAvailable() checks window.freighter
    (global as any).window = { freighter: {} };

    const adapter = createPreferredWalletAdapter();

    expect(adapter).toBeInstanceOf(FreighterAdapter);
  });

  it("falls back to AlbedoAdapter when only Albedo is available", () => {
    // AlbedoAdapter.isAvailable() checks window.albedo
    (global as any).window = { albedo: {} };

    const adapter = createPreferredWalletAdapter();

    expect(adapter).toBeInstanceOf(AlbedoAdapter);
  });

  it("falls back to XBullAdapter when only xBull is available", () => {
    (global as any).window = { xBull: {} };

    const adapter = createPreferredWalletAdapter();

    expect(adapter).toBeInstanceOf(XBullAdapter);
  });

  it("prefers Freighter over Albedo when both are available", () => {
    (global as any).window = { freighter: {}, albedo: {} };

    const adapter = createPreferredWalletAdapter();

    expect(adapter).toBeInstanceOf(FreighterAdapter);
  });

  it("throws when no supported wallet exists", () => {
    (global as any).window = {};

    expect(() => createPreferredWalletAdapter()).toThrow(
      "No supported wallet detected",
    );
  });

  it("throws with a message listing all supported wallets", () => {
    (global as any).window = {};

    expect(() => createPreferredWalletAdapter()).toThrow(
      /Freighter.*Albedo.*xBull/,
    );
  });
});
