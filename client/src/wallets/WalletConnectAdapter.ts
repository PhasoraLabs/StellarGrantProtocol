import { WalletAdapter } from "../types";

/**
 * WalletConnect v2 adapter for the StellarGrants SDK.
 *
 * Accepts a pre-initialized `@walletconnect/sign-client` instance so the host
 * application retains full control over project-ID configuration, QR-code
 * display, and URI sharing.
 *
 * @example
 * ```typescript
 * import SignClient from "@walletconnect/sign-client";
 *
 * const signClient = await SignClient.init({
 *   projectId: "YOUR_WALLETCONNECT_PROJECT_ID",
 *   metadata: {
 *     name: "StellarGrants",
 *     description: "Decentralised grant platform on Stellar",
 *     url: "https://stellargrants.io",
 *     icons: ["https://stellargrants.io/logo.png"],
 *   },
 * });
 *
 * const adapter = new WalletConnectAdapter(signClient);
 *
 * // Pair with a wallet (display uri as a QR code in your UI)
 * const { uri, approval } = await adapter.connect("Test SDF Network ; September 2015");
 * console.log("Scan this URI:", uri);
 * await approval(); // resolves once the user approves in their wallet
 *
 * const sdk = new StellarGrantsSDK({ wallet: adapter, ... });
 * ```
 */
export class WalletConnectAdapter implements WalletAdapter {
  readonly name = "WalletConnect";

  private readonly signClient: any;
  private session: any | null = null;

  /**
   * @param signClient An initialised `@walletconnect/sign-client` SignClient instance.
   */
  constructor(signClient: any) {
    if (!signClient) {
      throw new Error("WalletConnectAdapter: a SignClient instance is required.");
    }
    this.signClient = signClient;
    this._restoreSession();
  }

  /**
   * WalletConnect is pairing-based and does not require a browser extension,
   * so this always returns true.
   */
  isAvailable(): boolean {
    return true;
  }

  /**
   * Initiates a WalletConnect pairing.
   *
   * Returns a `uri` to display as a QR code and an `approval` callback that
   * resolves once the user approves the session in their wallet app.
   *
   * @param networkPassphrase Stellar network passphrase used to select the chain ID.
   */
  async connect(networkPassphrase: string): Promise<{ uri: string; approval: () => Promise<void> }> {
    const chainId = networkPassphrase.includes("Public")
      ? "stellar:pubnet"
      : "stellar:testnet";

    const { uri, approval } = await this.signClient.connect({
      requiredNamespaces: {
        stellar: {
          methods: ["stellar_signTransaction"],
          chains: [chainId],
          events: [],
        },
      },
    });

    return {
      uri: uri ?? "",
      approval: async () => {
        this.session = await approval();
      },
    };
  }

  /**
   * Disconnects the current WalletConnect session.
   */
  async disconnect(): Promise<void> {
    if (!this.session) return;
    try {
      await this.signClient.disconnect({
        topic: this.session.topic,
        reason: { code: 6000, message: "USER_DISCONNECTED" },
      });
    } finally {
      this.session = null;
    }
  }

  /** `true` when an active session exists. */
  get isConnected(): boolean {
    return this.session !== null;
  }

  async getPublicKey(): Promise<string> {
    this._requireSession();
    const accounts: string[] = this.session.namespaces?.stellar?.accounts ?? [];
    if (accounts.length === 0) {
      throw new Error("WalletConnect: no Stellar accounts found in the active session.");
    }
    // Accounts are formatted as "stellar:<network>:<G...address>"
    const parts = accounts[0].split(":");
    const pubkey = parts[parts.length - 1];
    if (!pubkey) {
      throw new Error("WalletConnect: could not parse public key from session accounts.");
    }
    return pubkey;
  }

  async signTransaction(txXdr: string, networkPassphrase: string): Promise<string> {
    this._requireSession();
    const chainId = networkPassphrase.includes("Public")
      ? "stellar:pubnet"
      : "stellar:testnet";

    const result = await this.signClient.request({
      topic: this.session.topic,
      chainId,
      request: {
        method: "stellar_signTransaction",
        params: { xdr: txXdr },
      },
    });

    if (typeof result === "string") return result;
    if (result?.signedXDR) return result.signedXDR;
    if (result?.xdr) return result.xdr;
    throw new Error("WalletConnect: unexpected response format from wallet.");
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private _restoreSession(): void {
    try {
      const sessions: any[] = this.signClient.session?.getAll?.() ?? [];
      if (sessions.length > 0) {
        this.session = sessions[sessions.length - 1];
      }
    } catch {
      this.session = null;
    }
  }

  private _requireSession(): void {
    if (!this.session) {
      throw new Error(
        "WalletConnect: no active session. Call connect() and wait for approval before signing.",
      );
    }
  }
}
