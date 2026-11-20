import { XBullAdapter } from "../../src/wallets/XBullAdapter";

const TESTNET_PASSPHRASE = "Test SDF Network ; September 2015";
const MAINNET_PASSPHRASE = "Public Global Stellar Network ; September 2015";

function makeXBullWindow(overrides?: {
  connect?: jest.Mock;
  signXDR?: jest.Mock;
}) {
  return {
    xBull: {
      connect: overrides?.connect ?? jest.fn(async () => "GXBULL_TEST_KEY"),
      signXDR: overrides?.signXDR ?? jest.fn(async () => "XBULL_SIGNED_XDR"),
    },
  };
}

afterEach(() => {
  delete (global as any).window;
});

describe("XBullAdapter.getPublicKey", () => {
  it("returns public key string directly from connect()", async () => {
    (global as any).window = makeXBullWindow();
    const adapter = new XBullAdapter();

    const key = await adapter.getPublicKey();

    expect(key).toBe("GXBULL_TEST_KEY");
  });

  it("extracts publicKey property when connect() returns an object", async () => {
    (global as any).window = makeXBullWindow({
      connect: jest.fn(async () => ({ publicKey: "GOBJ_KEY" })),
    });
    const adapter = new XBullAdapter();

    expect(await adapter.getPublicKey()).toBe("GOBJ_KEY");
  });

  it("extracts pubkey property when connect() returns an object with pubkey", async () => {
    (global as any).window = makeXBullWindow({
      connect: jest.fn(async () => ({ pubkey: "GPUBKEY_FIELD" })),
    });
    const adapter = new XBullAdapter();

    expect(await adapter.getPublicKey()).toBe("GPUBKEY_FIELD");
  });

  it("throws when xBull extension is not installed", async () => {
    (global as any).window = {};
    const adapter = new XBullAdapter();

    await expect(adapter.getPublicKey()).rejects.toThrow(
      "xBull Wallet extension is not installed",
    );
  });

  it("throws when window is undefined", async () => {
    delete (global as any).window;
    const adapter = new XBullAdapter();

    await expect(adapter.getPublicKey()).rejects.toThrow();
  });

  it("throws when connect() returns null", async () => {
    (global as any).window = makeXBullWindow({
      connect: jest.fn(async () => null),
    });
    const adapter = new XBullAdapter();

    await expect(adapter.getPublicKey()).rejects.toThrow(
      "xBull: connect() returned no response.",
    );
  });

  it("throws when connect() returns object without publicKey/pubkey", async () => {
    (global as any).window = makeXBullWindow({
      connect: jest.fn(async () => ({ other: "value" })),
    });
    const adapter = new XBullAdapter();

    await expect(adapter.getPublicKey()).rejects.toThrow(
      "xBull: could not retrieve public key",
    );
  });
});

describe("XBullAdapter.signTransaction", () => {
  it("calls signXDR with testnet network and returns signed XDR string", async () => {
    (global as any).window = makeXBullWindow();
    const adapter = new XBullAdapter();

    const result = await adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE);

    expect((global as any).window.xBull.signXDR).toHaveBeenCalledWith("TX_XDR", { network: "testnet" });
    expect(result).toBe("XBULL_SIGNED_XDR");
  });

  it("calls signXDR with public network for mainnet passphrase", async () => {
    (global as any).window = makeXBullWindow();
    const adapter = new XBullAdapter();

    await adapter.signTransaction("TX_XDR", MAINNET_PASSPHRASE);

    expect((global as any).window.xBull.signXDR).toHaveBeenCalledWith("TX_XDR", { network: "public" });
  });

  it("extracts xdr property when signXDR returns an object", async () => {
    (global as any).window = makeXBullWindow({
      signXDR: jest.fn(async () => ({ xdr: "OBJ_SIGNED_XDR" })),
    });
    const adapter = new XBullAdapter();

    const result = await adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE);

    expect(result).toBe("OBJ_SIGNED_XDR");
  });

  it("extracts signedXDR property from response object", async () => {
    (global as any).window = makeXBullWindow({
      signXDR: jest.fn(async () => ({ signedXDR: "SIGNED_XDR_FIELD" })),
    });
    const adapter = new XBullAdapter();

    const result = await adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE);

    expect(result).toBe("SIGNED_XDR_FIELD");
  });

  it("throws when xBull extension is not installed", async () => {
    (global as any).window = {};
    const adapter = new XBullAdapter();

    await expect(adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE)).rejects.toThrow(
      "xBull Wallet extension is not installed",
    );
  });

  it("throws when signXDR returns null", async () => {
    (global as any).window = makeXBullWindow({
      signXDR: jest.fn(async () => null),
    });
    const adapter = new XBullAdapter();

    await expect(adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE)).rejects.toThrow(
      "xBull: signXDR() returned no response.",
    );
  });

  it("throws when signXDR returns object without xdr/signedXDR", async () => {
    (global as any).window = makeXBullWindow({
      signXDR: jest.fn(async () => ({ other: "value" })),
    });
    const adapter = new XBullAdapter();

    await expect(adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE)).rejects.toThrow(
      "xBull: could not retrieve signed XDR",
    );
  });
});
