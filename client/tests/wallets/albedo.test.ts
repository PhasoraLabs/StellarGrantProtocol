import { AlbedoAdapter } from "../../src/wallets/AlbedoAdapter";

const TESTNET_PASSPHRASE = "Test SDF Network ; September 2015";
const MAINNET_PASSPHRASE = "Public Global Stellar Network ; September 2015";

function makeAlbedoWindow(overrides?: Partial<{ publicKey: jest.Mock; tx: jest.Mock }>) {
    return {
        albedo: {
            publicKey: overrides?.publicKey ?? jest.fn(async () => ({ pubkey: "GTEST_ALBEDO_KEY" })),
            tx: overrides?.tx ?? jest.fn(async () => ({ signed_envelope_xdr: "SIGNED_XDR" })),
        },
    };
}

afterEach(() => {
    // Clean up window mock
    delete (global as any).window;
});

describe("AlbedoAdapter.getPublicKey", () => {
    it("returns pubkey from window.albedo.publicKey", async () => {
        (global as any).window = makeAlbedoWindow();
        const adapter = new AlbedoAdapter();

        const key = await adapter.getPublicKey();

        expect(key).toBe("GTEST_ALBEDO_KEY");
        expect((global as any).window.albedo.publicKey).toHaveBeenCalledTimes(1);
    });

    it("caches the public key — window.albedo.publicKey called only once (Property 10)", async () => {
        (global as any).window = makeAlbedoWindow();
        const adapter = new AlbedoAdapter();

        const key1 = await adapter.getPublicKey();
        const key2 = await adapter.getPublicKey();
        const key3 = await adapter.getPublicKey();

        expect(key1).toBe(key2);
        expect(key2).toBe(key3);
        expect((global as any).window.albedo.publicKey).toHaveBeenCalledTimes(1);
    });

    it("throws when window.albedo is undefined", async () => {
        (global as any).window = {};
        const adapter = new AlbedoAdapter();

        await expect(adapter.getPublicKey()).rejects.toThrow("Albedo is not installed or available");
    });

    it("throws when window is undefined", async () => {
        delete (global as any).window;
        const adapter = new AlbedoAdapter();

        await expect(adapter.getPublicKey()).rejects.toThrow();
    });
});

describe("AlbedoAdapter.signTransaction", () => {
    it("calls window.albedo.tx with network: testnet for testnet passphrase", async () => {
        (global as any).window = makeAlbedoWindow();
        const adapter = new AlbedoAdapter();

        const result = await adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE);

        expect((global as any).window.albedo.tx).toHaveBeenCalledWith({
            xdr: "TX_XDR",
            network: "testnet",
        });
        expect(result).toBe("SIGNED_XDR");
    });

    it("calls window.albedo.tx with network: public for mainnet passphrase", async () => {
        (global as any).window = makeAlbedoWindow();
        const adapter = new AlbedoAdapter();

        await adapter.signTransaction("TX_XDR", MAINNET_PASSPHRASE);

        expect((global as any).window.albedo.tx).toHaveBeenCalledWith({
            xdr: "TX_XDR",
            network: "public",
        });
    });

    it("returns signed_envelope_xdr from albedo response", async () => {
        (global as any).window = makeAlbedoWindow({
            tx: jest.fn(async () => ({ signed_envelope_xdr: "MY_SIGNED_XDR" })),
        });
        const adapter = new AlbedoAdapter();

        const result = await adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE);

        expect(result).toBe("MY_SIGNED_XDR");
    });

    it("throws when window.albedo is undefined", async () => {
        (global as any).window = {};
        const adapter = new AlbedoAdapter();

        await expect(adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE)).rejects.toThrow(
            "Albedo is not installed or available",
        );
    });

    it("throws descriptive error when popup is blocked", async () => {
        (global as any).window = makeAlbedoWindow({
            tx: jest.fn(async () => {
                throw new Error("Popup blocked");
            }),
        });
        const adapter = new AlbedoAdapter();

        await expect(adapter.signTransaction("TX_XDR", TESTNET_PASSPHRASE)).rejects.toThrow(
            "Albedo popup was blocked or closed",
        );
    });
});
