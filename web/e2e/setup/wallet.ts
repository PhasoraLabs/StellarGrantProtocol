import { Page } from "@playwright/test";

interface FreighterMock {
  isConnected: () => Promise<boolean>;
  getPublicKey: () => Promise<string>;
  getNetwork: () => Promise<string>;
  signTransaction: (xdr: string) => Promise<string>;
}

export async function mockWalletConnected(
  page: Page,
  address = "GABC123456789012345678901234567890123456789012345678901234567890"
) {
  await page.addInitScript((addr) => {
    (window as unknown as { freighterApi: FreighterMock }).freighterApi = {
      isConnected: () => Promise.resolve(true),
      getPublicKey: () => Promise.resolve(addr),
      getNetwork: () => Promise.resolve("testnet"),
      signTransaction: (xdr: string) => Promise.resolve(xdr + "_signed"),
    };
  }, address);
}
