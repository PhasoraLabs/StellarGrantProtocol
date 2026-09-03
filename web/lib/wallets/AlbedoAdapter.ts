/**
 * AlbedoAdapter for use within the Next.js frontend.
 *
 * Albedo injects `window.albedo` when loaded. Signing happens via a popup
 * window — ensure popups are not blocked for your site.
 */

import type { WalletAdapter } from "./types";

type AlbedoNetwork = "testnet" | "public";

/** Minimal shape of the window.albedo global injected by Albedo. */
interface AlbedoGlobal {
  publicKey(params: Record<string, never>): Promise<{ pubkey?: string }>;
  tx(params: { xdr: string; network: AlbedoNetwork }): Promise<{ signed_envelope_xdr?: string }>;
}

export class AlbedoAdapter implements WalletAdapter {
  readonly name = "Albedo";
  readonly icon = "https://albedo.link/img/albedo-logo.svg";

  private publicKeyCache: string | null = null;
  private readonly network: AlbedoNetwork;

  constructor(networkPassphrase?: string) {
    this.network = this.resolveNetwork(networkPassphrase ?? "");
  }

  isAvailable(): boolean {
    return (
      typeof window !== "undefined" &&
      Boolean((window as Window & { albedo?: unknown }).albedo)
    );
  }

  async getPublicKey(): Promise<string> {
    if (this.publicKeyCache) return this.publicKeyCache;

    const albedo = (window as Window & { albedo?: AlbedoGlobal }).albedo;
    if (!albedo) throw new Error("Albedo is not installed or available");

    const response = await this.withPopupGuard<{ pubkey?: string }>(() =>
      albedo.publicKey({}),
    );
    if (!response?.pubkey) {
      throw new Error("Albedo did not return a public key.");
    }

    this.publicKeyCache = response.pubkey;
    return response.pubkey;
  }

  async signTransaction(txXdr: string, networkPassphrase: string): Promise<string> {
    const albedo = (window as Window & { albedo?: AlbedoGlobal }).albedo;
    if (!albedo) throw new Error("Albedo is not installed or available");

    const network = this.resolveNetwork(networkPassphrase || this.network);
    const response = await this.withPopupGuard<{ signed_envelope_xdr?: string }>(() =>
      albedo.tx({ xdr: txXdr, network }),
    );

    if (!response?.signed_envelope_xdr) {
      throw new Error("Albedo did not return a signed transaction envelope.");
    }
    return response.signed_envelope_xdr;
  }

  private resolveNetwork(networkPassphrase: string): AlbedoNetwork {
    return networkPassphrase.includes("Public") ? "public" : "testnet";
  }

  private async withPopupGuard<T>(cb: () => Promise<T>): Promise<T> {
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
