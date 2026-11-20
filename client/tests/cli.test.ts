import { resolveCliConfig } from "../src/cli";

describe("CLI config resolution", () => {
  const envBackup = { ...process.env };

  afterEach(() => {
    process.env = { ...envBackup };
  });

  it("throws when required connection credentials are missing", () => {
    delete process.env.CONTRACT_ID;
    delete process.env.RPC_URL;
    delete process.env.NETWORK_PASSPHRASE;

    expect(() => resolveCliConfig()).toThrow("Missing required connection credentials");
  });

  it("uses environment values", () => {
    process.env.CONTRACT_ID = "C123";
    process.env.RPC_URL = "https://rpc.example";
    process.env.NETWORK_PASSPHRASE = "Test SDF Network ; September 2015";

    const cfg = resolveCliConfig();

    expect(cfg.contractId).toBe("C123");
    expect(cfg.rpcUrl).toBe("https://rpc.example");
  });

  it("prefers explicit overrides", () => {
    process.env.CONTRACT_ID = "C123";
    process.env.RPC_URL = "https://rpc.example";
    process.env.NETWORK_PASSPHRASE = "Test SDF Network ; September 2015";

    const cfg = resolveCliConfig({
      contractId: "C999",
      rpcUrl: "https://rpc.override",
      networkPassphrase: "Public Global Stellar Network ; September 2015",
    });

    expect(cfg.contractId).toBe("C999");
    expect(cfg.rpcUrl).toBe("https://rpc.override");
    expect(cfg.networkPassphrase).toContain("Public");
  });
});
