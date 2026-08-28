export type StellarGrantsSigner = {
  getPublicKey(): Promise<string>;
  signTransaction(txXdr: string, networkPassphrase: string): Promise<string>;
};

export type WalletAdapter = StellarGrantsSigner & {
  /** Human-readable wallet name, e.g. "Freighter", "Albedo". */
  readonly name: string;

  /** URL to a wallet icon (SVG or PNG). Used by UI components. */
  readonly icon?: string;

  /**
   * Returns true if this wallet is available in the current environment.
   * Browser extension adapters check window globals; WalletConnect always returns true.
   * Called synchronously — no async detection needed.
   */
  isAvailable(): boolean;

  /** Optional: initiate a pairing flow (WalletConnect). */
  connect?(networkPassphrase: string): Promise<{ uri: string; approval: () => Promise<void> }>;

  /** Optional: tear down the session. */
  disconnect?(): Promise<void>;

  /** True when a session is active (WalletConnect). */
  isConnected?: boolean;
};

export type RetryConfig = {
  maxAttempts?: number;
  initialDelayMs?: number;
  backoffMultiplier?: number;
  maxDelayMs?: number;
  retryOnRateLimit?: boolean;
  retryOnTimeout?: boolean;
  retryOnNetworkError?: boolean;
  onRetry?: (attempt: number, error: Error, delayMs: number) => void;
};

export type StellarGrantsSDKConfig = {
  contractId: string;
  rpcUrl?: string;
  proxyUrl?: string;
  horizonUrl?: string;
  customHeaders?: Record<string, string>;
  networkPassphrase: string;
  signer?: StellarGrantsSigner;
  /**
   * Alias for `signer`. Accepts any WalletAdapter instance directly.
   * If both `wallet` and `signer` are provided, `wallet` takes precedence.
   *
   * @example
   * ```ts
   * import { FreighterAdapter } from "@stellargrants/client-sdk";
   * const sdk = new StellarGrantsSDK({ wallet: new FreighterAdapter(), ... });
   * ```
   */
  wallet?: WalletAdapter;
  defaultFee?: string;
};

export type GrantCreateInput = {
  owner: string;
  title: string;
  description: string;
  budget: bigint;
  deadline: bigint;
  milestoneCount: number;
};

export type GrantFundInput = {
  grantId: number;
  token: string;
  amount: bigint;
};

export type IpfsUploadConfig = {
  pinataJwt?: string;
  pinataApiKey?: string;
  pinataSecretKey?: string;
  metadataSchema?: IpfsMetadataSchemaName;
  name?: string;
  skipSchemaValidation?: boolean;
};

export type IpfsUploadResult = {
  cid: string;
  gatewayUrl: string;
};

export type IpfsMetadataSchemaName = "grant" | "milestone";

export type MilestoneSubmitInput = {
  grantId: number;
  milestoneIdx: number;
  proofHash: string;
};

export type MilestoneVoteInput = {
  grantId: number;
  milestoneIdx: number;
  approve: boolean;
};
