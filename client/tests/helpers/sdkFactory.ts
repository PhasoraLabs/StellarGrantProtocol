import { StellarGrantsSDK } from "../../src/StellarGrantsSDK";
import { StellarGrantsSDKConfig } from "../../src/types";
import { makeMockSigner } from "./mockSigner";
import { makeMockServer, MockServerState } from "./mockServer";

export const TEST_CONTRACT_ID = "CTEST_CONTRACT_ID_MOCK";
export const TEST_RPC_URL = "https://rpc.test.mock";
export const TEST_NETWORK_PASSPHRASE = "Test SDF Network ; September 2015";

/**
 * Creates a StellarGrantsSDK instance with the internal `server` field
 * replaced by a controllable mock. Returns the sdk, mock server, mock signer,
 * and the mutable state object for per-test configuration.
 */
export function makeSdk(overrides?: Partial<StellarGrantsSDKConfig>): {
    sdk: StellarGrantsSDK;
    mockServer: any;
    mockSigner: ReturnType<typeof makeMockSigner>;
    state: MockServerState;
} {
    const mockSigner = makeMockSigner();
    const { server: mockServer, state } = makeMockServer();

    const sdk = new StellarGrantsSDK({
        contractId: TEST_CONTRACT_ID,
        rpcUrl: TEST_RPC_URL,
        networkPassphrase: TEST_NETWORK_PASSPHRASE,
        signer: mockSigner,
        ...overrides,
    });

    // Inject mock server directly — avoids needing jest.mock at module level
    (sdk as any).server = mockServer;

    return { sdk, mockServer, mockSigner, state };
}
