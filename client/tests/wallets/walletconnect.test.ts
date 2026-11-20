import { WalletConnectAdapter } from "../../src/wallets/WalletConnectAdapter";

const TESTNET_PASSPHRASE = "Test SDF Network ; September 2015";
const MAINNET_PASSPHRASE = "Public Global Stellar Network ; September 2015";

const TESTNET_ACCOUNT = "stellar:testnet:GTEST_WC_PUBKEY";
const MAINNET_ACCOUNT = "stellar:pubnet:GMAIN_WC_PUBKEY";

function makeSignClient(overrides?: {
  sessions?: any[];
  connectResult?: { uri: string; approval: () => Promise<any> };
  requestResult?: any;
  disconnectMock?: jest.Mock;
}) {
  const sessions = overrides?.sessions ?? [];
  return {
    session: { getAll: jest.fn(() => sessions) },
    connect: jest.fn(async () =>
      overrides?.connectResult ?? {
        uri: "wc:mock-uri",
        approval: async () => ({
          topic: "mock-topic",
          namespaces: {
            stellar: { accounts: [TESTNET_ACCOUNT] },
          },
        }),
      },
    ),
    disconnect: overrides?.disconnectMock ?? jest.fn(async () => {}),
    request: jest.fn(async () => overrides?.requestResult ?? "SIGNED_XDR"),
  };
}

describe("WalletConnectAdapter constructor", () => {
  it("throws when no signClient is provided", () => {
    expect(() => new WalletConnectAdapter(null)).toThrow(
      "WalletConnectAdapter: a SignClient instance is required.",
    );
  });

  it("restores an existing session from the sign client", () => {
    const existingSession = {
      topic: "existing-topic",
      namespaces: { stellar: { accounts: [TESTNET_ACCOUNT] } },
    };
    const client = makeSignClient({ sessions: [existingSession] });
    const adapter = new WalletConnectAdapter(client);
    expect(adapter.isConnected).toBe(true);
  });

  it("starts with no session when none exist", () => {
    const client = makeSignClient({ sessions: [] });
    const adapter = new WalletConnectAdapter(client);
    expect(adapter.isConnected).toBe(false);
  });
});

describe("WalletConnectAdapter.connect", () => {
  it("connects on testnet and returns uri + approval", async () => {
    const client = makeSignClient();
    const adapter = new WalletConnectAdapter(client);

    const { uri, approval } = await adapter.connect(TESTNET_PASSPHRASE);

    expect(client.connect).toHaveBeenCalledWith({
      requiredNamespaces: {
        stellar: {
          methods: ["stellar_signTransaction"],
          chains: ["stellar:testnet"],
          events: [],
        },
      },
    });
    expect(uri).toBe("wc:mock-uri");

    await approval();
    expect(adapter.isConnected).toBe(true);
  });

  it("selects stellar:pubnet chain for mainnet passphrase", async () => {
    const client = makeSignClient({
      connectResult: {
        uri: "wc:main-uri",
        approval: async () => ({
          topic: "main-topic",
          namespaces: { stellar: { accounts: [MAINNET_ACCOUNT] } },
        }),
      },
    });
    const adapter = new WalletConnectAdapter(client);

    await adapter.connect(MAINNET_PASSPHRASE);

    expect(client.connect).toHaveBeenCalledWith(
      expect.objectContaining({
        requiredNamespaces: expect.objectContaining({
          stellar: expect.objectContaining({ chains: ["stellar:pubnet"] }),
        }),
      }),
    );
  });
});

describe("WalletConnectAdapter.getPublicKey", () => {
  it("throws when no session is active", async () => {
    const client = makeSignClient({ sessions: [] });
    const adapter = new WalletConnectAdapter(client);

    await expect(adapter.getPublicKey()).rejects.toThrow(
      "WalletConnect: no active session.",
    );
  });

  it("parses public key from testnet account string", async () => {
    const client = makeSignClient();
    const adapter = new WalletConnectAdapter(client);
    const { approval } = await adapter.connect(TESTNET_PASSPHRASE);
    await approval();

    const key = await adapter.getPublicKey();

    expect(key).toBe("GTEST_WC_PUBKEY");
  });

  it("parses public key from mainnet account string", async () => {
    const client = makeSignClient({
      connectResult: {
        uri: "wc:x",
        approval: async () => ({
          topic: "t",
          namespaces: { stellar: { accounts: [MAINNET_ACCOUNT] } },
        }),
      },
    });
    const adapter = new WalletConnectAdapter(client);
    const { approval } = await adapter.connect(MAINNET_PASSPHRASE);
    await approval();

    expect(await adapter.getPublicKey()).toBe("GMAIN_WC_PUBKEY");
  });

  it("throws when session has no stellar accounts", async () => {
    const client = makeSignClient({
      connectResult: {
        uri: "wc:x",
        approval: async () => ({
          topic: "t",
          namespaces: { stellar: { accounts: [] } },
        }),
      },
    });
    const adapter = new WalletConnectAdapter(client);
    const { approval } = await adapter.connect(TESTNET_PASSPHRASE);
    await approval();

    await expect(adapter.getPublicKey()).rejects.toThrow(
      "WalletConnect: no Stellar accounts found",
    );
  });
});

describe("WalletConnectAdapter.signTransaction", () => {
  it("throws when no session is active", async () => {
    const client = makeSignClient();
    const adapter = new WalletConnectAdapter(client);

    await expect(adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE)).rejects.toThrow(
      "WalletConnect: no active session.",
    );
  });

  it("calls signClient.request with stellar_signTransaction on testnet", async () => {
    const client = makeSignClient({ requestResult: "SIGNED_XDR" });
    const adapter = new WalletConnectAdapter(client);
    const { approval } = await adapter.connect(TESTNET_PASSPHRASE);
    await approval();

    const result = await adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE);

    expect(client.request).toHaveBeenCalledWith({
      topic: "mock-topic",
      chainId: "stellar:testnet",
      request: { method: "stellar_signTransaction", params: { xdr: "TX_XDR" } },
    });
    expect(result).toBe("SIGNED_XDR");
  });

  it("uses stellar:pubnet chain for mainnet passphrase", async () => {
    const client = makeSignClient({
      connectResult: {
        uri: "wc:x",
        approval: async () => ({
          topic: "main-topic",
          namespaces: { stellar: { accounts: [MAINNET_ACCOUNT] } },
        }),
      },
      requestResult: "MAIN_SIGNED_XDR",
    });
    const adapter = new WalletConnectAdapter(client);
    const { approval } = await adapter.connect(MAINNET_PASSPHRASE);
    await approval();

    await adapter.signTransaction("TX_XDR", MAINNET_PASSPHRASE);

    expect(client.request).toHaveBeenCalledWith(
      expect.objectContaining({ chainId: "stellar:pubnet" }),
    );
  });

  it("extracts signedXDR from object response", async () => {
    const client = makeSignClient({ requestResult: { signedXDR: "OBJ_SIGNED_XDR" } });
    const adapter = new WalletConnectAdapter(client);
    const { approval } = await adapter.connect(TESTNET_PASSPHRASE);
    await approval();

    const result = await adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE);
    expect(result).toBe("OBJ_SIGNED_XDR");
  });

  it("extracts xdr from object response", async () => {
    const client = makeSignClient({ requestResult: { xdr: "XDR_FIELD" } });
    const adapter = new WalletConnectAdapter(client);
    const { approval } = await adapter.connect(TESTNET_PASSPHRASE);
    await approval();

    const result = await adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE);
    expect(result).toBe("XDR_FIELD");
  });

  it("throws on unexpected response format", async () => {
    const client = makeSignClient({ requestResult: { unexpected: true } });
    const adapter = new WalletConnectAdapter(client);
    const { approval } = await adapter.connect(TESTNET_PASSPHRASE);
    await approval();

    await expect(adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE)).rejects.toThrow(
      "WalletConnect: unexpected response format",
    );
  });
});

describe("WalletConnectAdapter.disconnect", () => {
  it("disconnects and clears the session", async () => {
    const disconnectMock = jest.fn(async () => {});
    const client = makeSignClient({ disconnectMock });
    const adapter = new WalletConnectAdapter(client);
    const { approval } = await adapter.connect(TESTNET_PASSPHRASE);
    await approval();
    expect(adapter.isConnected).toBe(true);

    await adapter.disconnect();

    expect(disconnectMock).toHaveBeenCalledWith({
      topic: "mock-topic",
      reason: { code: 6000, message: "USER_DISCONNECTED" },
    });
    expect(adapter.isConnected).toBe(false);
  });

  it("is a no-op when no session exists", async () => {
    const disconnectMock = jest.fn(async () => {});
    const client = makeSignClient({ sessions: [], disconnectMock });
    const adapter = new WalletConnectAdapter(client);

    await expect(adapter.disconnect()).resolves.toBeUndefined();
    expect(disconnectMock).not.toHaveBeenCalled();
  });
});
