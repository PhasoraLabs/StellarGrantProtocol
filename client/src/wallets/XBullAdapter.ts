import { WalletAdapter } from "../types";

/**
 * Adapter for the xBull Wallet browser extension.
 *
 * xBull injects `window.xBull` when the extension is installed.
 * If the extension is absent the adapter throws a descriptive error so the
 * caller can prompt the user to install it.
 *
 * @example
 * ```typescript
 * const adapter = new XBullAdapter();
 * const sdk = new StellarGrantsSDK({ wallet: adapter, ... });
 * ```
 */
export class XBullAdapter implements WalletAdapter {
  readonly name = "xBull";
  readonly icon = "https://xbull.app/assets/icons/icon-192x192.png";

  /**
   * Returns true when the xBull extension is installed and has injected its global.
   */
  isAvailable(): boolean {
    return typeof window !== "undefined" && Boolean((window as any).xBull);
  }
  async getPublicKey(): Promise<string> {
    const xBull = this._getExtension();

    const response = await xBull.connect();
    if (!response) {
      throw new Error("xBull: connect() returned no response.");
    }

    // xBull.connect() resolves to either a plain public-key string or an
    // object with a publicKey / pubkey property depending on the extension version.
    if (typeof response === "string") return response;
    const pubkey = response.publicKey ?? response.pubkey;
    if (!pubkey) {
      throw new Error("xBull: could not retrieve public key from connect() response.");
    }
    return pubkey;
  }

  async signTransaction(txXdr: string, networkPassphrase: string): Promise<string> {
    const xBull = this._getExtension();

    const network = networkPassphrase.includes("Public") ? "public" : "testnet";
    const response = await xBull.signXDR(txXdr, { network });

    if (!response) {
      throw new Error("xBull: signXDR() returned no response.");
    }

    // signXDR resolves to either a signed-XDR string or an object with xdr property.
    if (typeof response === "string") return response;
    const signedXdr = response.xdr ?? response.signedXDR;
    if (!signedXdr) {
      throw new Error("xBull: could not retrieve signed XDR from signXDR() response.");
    }
    return signedXdr;
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private _getExtension(): any {
    const xBull = (window as any)?.xBull;
    if (!xBull) {
      throw new Error(
        "xBull Wallet extension is not installed or not accessible. " +
          "Please install xBull from https://xbull.app and reload the page.",
      );
    }
    return xBull;
  }
}
