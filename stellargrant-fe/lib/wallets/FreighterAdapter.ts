/**
 * Re-export FreighterAdapter for use within the Next.js frontend.
 *
 * The adapter uses @stellar/freighter-api (already a dependency of this package)
 * and satisfies the WalletAdapter interface defined in @/lib/wallets/types.
 */

import {
  isAllowed,
  setAllowed,
  getAddress,
  getNetworkDetails,
  signTransaction as freighterSignTransaction,
} from "@stellar/freighter-api";
import type { WalletAdapter } from "./types";

export class FreighterAdapter implements WalletAdapter {
  readonly name = "Freighter";
  readonly icon = "https://freighter.app/favicon.ico";

  isAvailable(): boolean {
    return (
      typeof window !== "undefined" &&
      Boolean((window as Window & { freighter?: unknown }).freighter)
    );
  }

  async getPublicKey(): Promise<string> {
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

  async signTransaction(txXdr: string, networkPassphrase: string): Promise<string> {
    const networkResult = await getNetworkDetails();
    if (networkResult.networkPassphrase !== networkPassphrase) {
      throw new Error(
        `Freighter is on wrong network. Expected: ${networkPassphrase}`,
      );
    }

    const result = await freighterSignTransaction(txXdr, { networkPassphrase });
    if (result.error) {
      throw new Error(result.error);
    }
    if (!result.signedTxXdr) {
      throw new Error("Failed to sign transaction with Freighter");
    }
    return result.signedTxXdr;
  }
}
