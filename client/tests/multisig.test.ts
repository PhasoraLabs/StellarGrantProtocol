import { makeSdk } from "./helpers/sdkFactory";

jest.mock("@stellar/stellar-sdk", () => {
  const actual = jest.requireActual("@stellar/stellar-sdk");
  return {
    ...actual,
    Contract: jest.fn().mockImplementation(() => ({
      call: jest.fn(),
    })),
  };
});

describe("multisig pipeline", () => {
  it("getAccountSigners fetches signers and thresholds from RPC", async () => {
    const { sdk, mockServer } = makeSdk();
    mockServer.getAccount = async () => ({
      signers: [
        { key: "GB7...", weight: 2 },
        { key: "GB8...", weight: 1 },
      ],
      thresholds: {
        low_threshold: 1,
        med_threshold: 3,
        high_threshold: 5,
      },
    });

    const result = await sdk.getAccountSigners("GBTEST");

    expect(result.signers).toHaveLength(2);
    expect(result.signers[0].key).toBe("GB7...");
    expect(result.signers[0].weight).toBe(2);
    expect(result.thresholds.low_threshold).toBe(1);
    expect(result.thresholds.med_threshold).toBe(3);
    expect(result.thresholds.high_threshold).toBe(5);
  });

  it("meetsThreshold checks against low threshold", () => {
    const { sdk } = makeSdk();
    const thresholds = { low_threshold: 1, med_threshold: 3, high_threshold: 5 };

    expect(sdk.meetsThreshold(1, thresholds, "low")).toBe(true);
    expect(sdk.meetsThreshold(0, thresholds, "low")).toBe(false);
  });

  it("meetsThreshold checks against medium threshold", () => {
    const { sdk } = makeSdk();
    const thresholds = { low_threshold: 1, med_threshold: 3, high_threshold: 5 };

    expect(sdk.meetsThreshold(3, thresholds, "medium")).toBe(true);
    expect(sdk.meetsThreshold(2, thresholds, "medium")).toBe(false);
  });

  it("meetsThreshold checks against high threshold", () => {
    const { sdk } = makeSdk();
    const thresholds = { low_threshold: 1, med_threshold: 3, high_threshold: 5 };

    expect(sdk.meetsThreshold(5, thresholds, "high")).toBe(true);
    expect(sdk.meetsThreshold(4, thresholds, "high")).toBe(false);
  });

  it("pendingXdrStore is available as a public readonly property", () => {
    const { sdk } = makeSdk();

    expect(sdk.pendingXdrStore).toBeDefined();
    sdk.pendingXdrStore.save("test-id", "TEST_XDR");
    expect(sdk.pendingXdrStore.get("test-id")).toBe("TEST_XDR");
    sdk.pendingXdrStore.delete("test-id");
    expect(sdk.pendingXdrStore.get("test-id")).toBeNull();
  });
});
