import { WalletAdapter } from "../types";

type AlbedoNetwork = "testnet" | "public";

/**
 * Adapter for the Albedo web-based Stellar signer.
 *
 * Albedo injects `window.albedo` when loaded. Unlike browser extensions,
 * Albedo works via popup windows — ensure popups are not blocked for your site.
 *
 * @example
 * ```typescript
 * const adapter = new AlbedoAdapter();
 * const sdk = new StellarGrantsSDK({ wallet: adapter, ... });
 * ```
 */
export class AlbedoAdapter implements WalletAdapter {
  readonly name = "Albedo";
  readonly icon = "https://albedo.link/img/albedo-logo.svg";

  private publicKeyCache: string | null = null;
  private network: AlbedoNetwork = "testnet";

  /**
   * Returns true when the Albedo global is available in the current environment.
   */
  isAvailable(): boolean {
    return typeof window !== "undefined" && Boolean((window as any).albedo);
  }

  constructor(networkPassphrase?: string) {
    this.network = this.resolveNetwork(networkPassphrase ?? "");
  }

  async getPublicKey(): Promise<string> {
    if (this.publicKeyCache) return this.publicKeyCache;
    const albedo = (window as any).albedo;
    if (!albedo) throw new Error("Albedo is not installed or available");

    const response = await this.executeWithPopupGuard<{ pubkey?: string }>(() => albedo.publicKey({}));
    if (!response?.pubkey) {
      throw new Error("Albedo did not return a public key.");
    }

    this.publicKeyCache = response.pubkey;
    return response.pubkey;
  }

  async signTransaction(txXdr: string, networkPassphrase: string): Promise<string> {
    const albedo = (window as any).albedo;
    if (!albedo) throw new Error("Albedo is not installed or available");

    const network = this.resolveNetwork(networkPassphrase || this.network);

    const response = await this.executeWithPopupGuard<{ signed_envelope_xdr?: string }>(() => albedo.tx({
      xdr: txXdr,
      network,
    }));

    if (!response?.signed_envelope_xdr) {
      throw new Error("Albedo did not return a signed transaction envelope.");
    }

    return response.signed_envelope_xdr;
  }

  private resolveNetwork(networkPassphrase: string): AlbedoNetwork {
    if (networkPassphrase.includes("Public")) {
      return "public";
    }
    // Albedo prompts currently support testnet/public. Futurenet and custom
    // passphrases are mapped to testnet for compatibility.
    return "testnet";
  }

  private async executeWithPopupGuard<T>(cb: () => Promise<T>): Promise<T> {
    try {
      return await cb();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (/popup|blocked|denied|closed|cancel/i.test(message)) {
        throw new Error(
          "Albedo popup was blocked or closed. Enable popups for this site and try again.",
        );
      }
      throw error;
    }
  }
}
