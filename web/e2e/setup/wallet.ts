import { Page } from "@playwright/test";

export async function mockWalletConnected(
  page: Page,
  address = "GCK4LZF4RXGPTVCQFSAZ5RYS7CY26BSSML5TJLSBVWUNFJDTNGJ2LOWH"
) {
  await page.addInitScript((addr) => {
    // Hydrate the app's persisted wallet store. The Freighter v6 API talks to
    // the extension through postMessage, so assigning the old freighterApi
    // global does not mock the adapter used by the app.
    window.localStorage.setItem(
      "stellar-grants-wallet",
      JSON.stringify({
        state: { address: addr, walletType: null, network: "testnet" },
        version: 0,
      }),
    );
  }, address);
}
