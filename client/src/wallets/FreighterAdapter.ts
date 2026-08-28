import {
  isAllowed,
  setAllowed,
  getAddress,
  getNetworkDetails,
  signTransaction as freighterSignTransaction,
} from "@stellar/freighter-api";
import { WalletAdapter } from "../types";

/**
 * Adapter for the Freighter browser extension wallet.
 *
 * Uses the official `@stellar/freighter-api` npm package (v2 API).
 * Freighter must be installed as a browser extension; this adapter will
 * throw a descriptive error if the extension is absent.
 *
 * @example
 * ```typescript
 * const adapter = new FreighterAdapter();
 * const sdk = new StellarGrantsSDK({ wallet: adapter, ... });
 * ```
 */
export class FreighterAdapter implements WalletAdapter {
  readonly name = "Freighter";
  readonly icon = "https://freighter.app/favicon.ico";

  /**
   * Returns true when the Freighter extension is installed and has injected
   * its global into the page. Safe to call synchronously.
   */
  isAvailable(): boolean {
    return typeof window !== "undefined" && Boolean((window as any).freighter);
  }

  /**
   * Requests permission from Freighter (if not already granted) and returns
   * the user's active public key.
   */
  async getPublicKey(): Promise<string> {
    // Request permission if not already granted
    const access = await isAllowed();
    if (!access.isAllowed) {
      const permission = await setAllowed();
      if (!permission.isAllowed) {
        throw new Error("Freighter permission denied");
      }
    }

    const result = await getAddress();
    if (result.error) {
      throw new Error(result.error);
    }
    if (!result.address) {
      throw new Error("No public key returned by Freighter");
    }
    return result.address;
  }

  /**
   * Signs a transaction XDR string using Freighter.
   * Validates that Freighter is connected to the expected network before signing.
   *
   * @param txXdr - The base64-encoded transaction XDR to sign.
   * @param networkPassphrase - The Stellar network passphrase (used for network validation).
   * @returns The signed transaction XDR string.
   */
  async signTransaction(txXdr: string, networkPassphrase: string): Promise<string> {
    // Validate that Freighter is on the expected network
    const networkResult = await getNetworkDetails();
    if (networkResult.networkPassphrase !== networkPassphrase) {
      throw new Error(
        `Freighter is on wrong network. Expected: ${networkPassphrase}`,
      );
    }

    const result = await freighterSignTransaction(txXdr, {
      networkPassphrase,
    });

    if (result.error) {
      throw new Error(result.error);
    }
    if (!result.signedTxXdr) {
      throw new Error("Failed to sign transaction with Freighter");
    }
    return result.signedTxXdr;
  }
}
