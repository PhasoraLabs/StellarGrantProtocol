import { WalletAdapter } from "../../src/types";

export type MockSigner = {
    getPublicKey: jest.Mock<Promise<string>>;
    signTransaction: jest.Mock<Promise<string>>;
} & WalletAdapter;

export function makeMockSigner(): MockSigner {
    return {
        name: "MockSigner",
        isAvailable: () => true,
        getPublicKey: jest.fn(async () => "GB3KJPLFUYN5VL6R3GU3EGCGVCKFDSD7BEDX42HWG5BWFKB3KQGJJRMA"),
        signTransaction: jest.fn(async (_txXdr: string, _passphrase: string) => "SIGNED_XDR_STRING"),
    };
}
