import { WalletAdapter } from "../types";
import { AlbedoAdapter } from "./AlbedoAdapter";
import { FreighterAdapter } from "./FreighterAdapter";
import { XBullAdapter } from "./XBullAdapter";

/**
 * Returns the first available wallet adapter by checking each adapter's
 * `isAvailable()` method in priority order: Freighter → Albedo → xBull.
 *
 * Useful for apps that want automatic wallet detection without requiring
 * the user to explicitly choose a wallet.
 *
 * @param networkPassphrase - Optional Stellar network passphrase forwarded to
 *   the AlbedoAdapter constructor for network resolution.
 * @throws If no supported wallet is detected in the current environment.
 *
 * @example
 * ```typescript
 * const adapter = createPreferredWalletAdapter(Networks.TESTNET);
 * const sdk = new StellarGrantsSDK({ wallet: adapter, ... });
 * ```
 */
export function createPreferredWalletAdapter(networkPassphrase?: string): WalletAdapter {
  const candidates: WalletAdapter[] = [
    new FreighterAdapter(),
    new AlbedoAdapter(networkPassphrase),
    new XBullAdapter(),
  ];

  const found = candidates.find((adapter) => adapter.isAvailable());
  if (!found) {
    throw new Error(
      "No supported wallet detected. Install Freighter, Albedo, or xBull and refresh.",
    );
  }
  return found;
}
