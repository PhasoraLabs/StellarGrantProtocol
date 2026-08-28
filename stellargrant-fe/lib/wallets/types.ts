/**
 * WalletAdapter interface for the frontend.
 *
 * Mirrors the SDK's WalletAdapter type so the frontend can type-check
 * adapter instances without importing from the SDK's source tree.
 *
 * Any object satisfying this interface can be passed to `useWallet`
 * or used directly with `StellarGrantsSDK`.
 */
export interface WalletAdapter {
  /** Human-readable wallet name, e.g. "Freighter", "Albedo". */
  readonly name: string;

  /** URL to a wallet icon (SVG or PNG). */
  readonly icon?: string;

  /**
   * Returns true if this wallet is available in the current environment.
   * Called synchronously — no async detection needed.
   */
  isAvailable(): boolean;

  /** Returns the user's active Stellar public key (G... address). */
  getPublicKey(): Promise<string>;

  /**
   * Signs a transaction XDR and returns the signed XDR string.
   * @param txXdr - Base64-encoded transaction XDR.
   * @param networkPassphrase - Stellar network passphrase.
   */
  signTransaction(txXdr: string, networkPassphrase: string): Promise<string>;

  /** Optional: tear down the session (e.g. WalletConnect). */
  disconnect?(): Promise<void>;
}
