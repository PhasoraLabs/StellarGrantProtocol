export type StellarGrantsSigner = {
  getPublicKey(): Promise<string>;
  signTransaction(txXdr: string, networkPassphrase: string): Promise<string>;
};

export type WalletAdapter = StellarGrantsSigner & {
  connect?(networkPassphrase: string): Promise<{ uri: string; approval: () => Promise<void> }>;
  disconnect?(): Promise<void>;
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
