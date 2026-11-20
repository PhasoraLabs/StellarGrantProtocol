# API Reference

Complete API documentation for the StellarGrants SDK.

## Table of Contents

- [StellarGrantsSDK](#stellargrantssdk)
- [IPFS Functions](#ipfs-functions)
- [Transaction Utilities](#transaction-utilities)
- [Wallet Adapters](#wallet-adapters)
- [Types](#types)
- [Errors](#errors)

## StellarGrantsSDK

Main SDK class for interacting with the StellarGrants contract.

### Constructor

```typescript
new StellarGrantsSDK(config: StellarGrantsSDKConfig)
```

#### Parameters

- `config` - SDK configuration object

#### Example

```typescript
const sdk = new StellarGrantsSDK({
  contractId: "CXXX...",
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
  wallet: new FreighterAdapter(),
});
```

### Grant Methods

#### `grantCreate()`

Create a new grant on the blockchain.

```typescript
async grantCreate(
  input: GrantCreateInput,
  options?: {
    feePriority?: "low" | "medium" | "high";
    simulatedFee?: string;
    footprint?: any;
    ipfsConfig?: IpfsUploadConfig;
    uploadMetadata?: boolean;
  }
): Promise<unknown>
```

**Parameters:**

- `input.owner` - Stellar address of grant owner
- `input.title` - Grant title (max 100 chars recommended)
- `input.description` - Grant description or IPFS CID
- `input.budget` - Total budget in stroops (bigint)
- `input.deadline` - Unix timestamp deadline (bigint)
- `input.milestoneCount` - Number of milestones (1-10)

**Options:**

- `feePriority` - Fee level: "low", "medium", or "high"
- `uploadMetadata` - Auto-upload description to IPFS
- `ipfsConfig` - IPFS configuration for auto-upload

**Returns:** Transaction result

**Example:**

```typescript
const result = await sdk.grantCreate({
  owner: "GABC...",
  title: "Ocean Cleanup",
  description: "Coastal cleanup initiative",
  budget: BigInt("5000000000"),
  deadline: BigInt(Date.now() / 1000 + 86400 * 90),
  milestoneCount: 4,
});
```

#### `grantFund()`

Fund an existing grant.

```typescript
async grantFund(
  input: GrantFundInput,
  options?: {
    feePriority?: "low" | "medium" | "high";
    simulatedFee?: string;
    footprint?: any;
  }
): Promise<unknown>
```

**Parameters:**

- `input.grantId` - Grant identifier
- `input.token` - Token contract address
- `input.amount` - Amount to fund in token units (bigint)

**Example:**

```typescript
await sdk.grantFund({
  grantId: 1,
  token: "CDLZFC...", // USDC
  amount: BigInt("1000000000"),
});
```

#### `grantGet()`

Retrieve grant details.

```typescript
async grantGet(
  grantId: number,
  options?: {
    fetchIpfsMetadata?: boolean;
    ipfsGateways?: string[];
  }
): Promise<unknown>
```

**Parameters:**

- `grantId` - Grant identifier
- `options.fetchIpfsMetadata` - Auto-fetch IPFS metadata
- `options.ipfsGateways` - Custom IPFS gateways

**Example:**

```typescript
const grant = await sdk.grantGet(1, {
  fetchIpfsMetadata: true,
});
```

### Milestone Methods

#### `milestoneSubmit()`

Submit proof for a milestone.

```typescript
async milestoneSubmit(
  input: MilestoneSubmitInput,
  options?: {
    feePriority?: "low" | "medium" | "high";
    simulatedFee?: string;
  }
): Promise<unknown>
```

**Parameters:**

- `input.grantId` - Grant identifier
- `input.milestoneIdx` - Milestone index (0-based)
- `input.proofHash` - IPFS CID or proof hash

**Example:**

```typescript
await sdk.milestoneSubmit({
  grantId: 1,
  milestoneIdx: 0,
  proofHash: "QmX7Y8Z...",
});
```

#### `milestoneVote()`

Vote on a milestone submission.

```typescript
async milestoneVote(
  input: MilestoneVoteInput,
  options?: {
    feePriority?: "low" | "medium" | "high";
    simulatedFee?: string;
  }
): Promise<unknown>
```

**Parameters:**

- `input.grantId` - Grant identifier
- `input.milestoneIdx` - Milestone index
- `input.approve` - true to approve, false to reject

**Example:**

```typescript
await sdk.milestoneVote({
  grantId: 1,
  milestoneIdx: 0,
  approve: true,
});
```

#### `milestoneGet()`

Get milestone details.

```typescript
async milestoneGet(
  grantId: number,
  milestoneIdx: number
): Promise<unknown>
```

**Example:**

```typescript
const milestone = await sdk.milestoneGet(1, 0);
```

### Transaction Methods

#### `waitForTransaction()`

Poll for transaction confirmation.

```typescript
async waitForTransaction(
  hash: string,
  options?: WaitForTransactionOptions
): Promise<TransactionResult>
```

**Options:**

- `pollIntervalMs` - Polling interval (default: 3000)
- `timeoutMs` - Timeout in milliseconds (default: 60000)
- `maxNetworkRetries` - Max retry attempts (default: 3)
- `onStatusChange` - Status change callback
- `onPoll` - Poll attempt callback
- `signal` - AbortSignal for cancellation

**Example:**

```typescript
const result = await sdk.waitForTransaction(txHash, {
  pollIntervalMs: 2000,
  timeoutMs: 120000,
  onStatusChange: (status) => console.log(status),
});
```

#### `estimateFees()`

Estimate transaction fees with network data.

```typescript
async estimateFees(
  method: string,
  args: xdr.ScVal[],
  options?: {
    horizonUrl?: string;
    feePriority?: "low" | "medium" | "high";
    simulatedFee?: string;
  }
): Promise<FeeEstimate>
```

**Example:**

```typescript
const fees = await sdk.estimateFees("grant_create", args);
console.log("Recommended fee:", fees.medium);
```

### IPFS Methods

#### `uploadMetadataToIPFS()`

Upload metadata to IPFS via SDK instance.

```typescript
async uploadMetadataToIPFS(
  metadata: Record<string, unknown>,
  config: IpfsUploadConfig
): Promise<IpfsUploadResult>
```

**Example:**

```typescript
const { cid } = await sdk.uploadMetadataToIPFS(
  { title: "Grant", description: "..." },
  { pinataJwt: process.env.PINATA_JWT }
);
```

#### `fetchMetadataFromIPFS()`

Fetch metadata from IPFS via SDK instance.

```typescript
async fetchMetadataFromIPFS(
  cid: string,
  gateways?: string[]
): Promise<Record<string, unknown>>
```

**Example:**

```typescript
const metadata = await sdk.fetchMetadataFromIPFS("QmX7Y8Z...");
```

### Utility Methods

#### `checkCompatibility()`

Check SDK and contract version compatibility.

```typescript
async checkCompatibility(): Promise<{
  compatible: boolean;
  sdkVersion: number;
  contractVersion: number | null;
  warning?: string;
}>
```

**Example:**

```typescript
const compat = await sdk.checkCompatibility();
if (!compat.compatible) {
  console.warn(compat.warning);
}
```

#### `subscribeToEvents()`

Subscribe to contract events.

```typescript
subscribeToEvents(
  callback: (event: any) => void,
  options?: {
    eventName?: string;
    startCursor?: string;
    pollIntervalMs?: number;
    maxRetries?: number;
    onError?: (error: any) => void;
    onStatusChange?: (status: string) => void;
  }
): () => void
```

**Example:**

```typescript
const unsubscribe = sdk.subscribeToEvents(
  (event) => console.log("Event:", event),
  { eventName: "grant_created" }
);

// Later: unsubscribe()
```

## IPFS Functions

Standalone functions for IPFS operations.

### `uploadMetadataToIPFS()`

```typescript
async function uploadMetadataToIPFS(
  metadata: Record<string, unknown>,
  config: IpfsUploadConfig
): Promise<IpfsUploadResult>
```

**Config:**

- `pinataJwt` - Pinata JWT token
- `pinataApiKey` - Pinata API key (alternative)
- `pinataSecretKey` - Pinata secret key (with API key)
- `metadataSchema` - Schema name for validation
- `name` - Metadata name for Pinata
- `skipSchemaValidation` - Skip validation

**Example:**

```typescript
import { uploadMetadataToIPFS } from "@stellargrants/client-sdk";

const { cid, gatewayUrl } = await uploadMetadataToIPFS(
  { title: "Grant", description: "..." },
  { pinataJwt: "your_jwt" }
);
```

### `fetchMetadataFromIPFS()`

```typescript
async function fetchMetadataFromIPFS(
  cid: string,
  gateways?: string[]
): Promise<Record<string, unknown>>
```

**Example:**

```typescript
import { fetchMetadataFromIPFS } from "@stellargrants/client-sdk";

const metadata = await fetchMetadataFromIPFS("QmX7Y8Z...", [
  "https://gateway.pinata.cloud/ipfs/",
  "https://ipfs.io/ipfs/",
]);
```

## Transaction Utilities

### TransactionTracker

Event-driven transaction lifecycle tracking.

```typescript
class TransactionTracker {
  on<T>(event: T, listener: EventListener<T>): () => void;
  off<T>(event: T, listener: EventListener<T>): void;
  track<T>(txFn: () => Promise<any>, predictedState?: T): Promise<string>;
  getTransaction(txId: string): OptimisticUpdate | undefined;
  getAllTransactions(): OptimisticUpdate[];
  clear(txId: string): void;
  clearAll(): void;
}
```

**Events:**

- `signed` - Transaction signed
- `submitted` - Transaction submitted
- `confirmed` - Transaction confirmed
- `failed` - Transaction failed
- `stageChange` - Stage changed

**Example:**

```typescript
const tracker = new TransactionTracker();

tracker.on("confirmed", (txId, result) => {
  console.log("Confirmed:", result);
});

const txId = await tracker.track(async () => {
  return await sdk.grantCreate(input);
});
```

### OptimisticStateManager

Manage optimistic UI updates with rollback.

```typescript
class OptimisticStateManager {
  predictGrantCreate(input: GrantCreateInput): OptimisticGrant;
  predictGrantFund(current: OptimisticGrant, input: GrantFundInput): OptimisticGrant;
  predictMilestoneSubmit(current: OptimisticMilestone, proofHash: string): OptimisticMilestone;
  predictMilestoneVote(current: OptimisticMilestone, approve: boolean, votes: Votes, threshold: number): OptimisticMilestone;
  apply(txId: string, predictedState: any, type: string, previousState?: any): void;
  commit(txId: string, actualState: any): void;
  rollback(txId: string): any | undefined;
  getPredictedState(txId: string): any | undefined;
  clearAll(): void;
}
```

**Example:**

```typescript
const manager = new OptimisticStateManager();

const predicted = manager.predictGrantCreate(input);
manager.apply("tx_123", predicted, "grant_create");

try {
  const result = await sdk.grantCreate(input);
  manager.commit("tx_123", result);
} catch (error) {
  manager.rollback("tx_123");
}
```

## Wallet Adapters

### FreighterAdapter

```typescript
const wallet = new FreighterAdapter();
```

### AlbedoAdapter

```typescript
const wallet = new AlbedoAdapter();
```

### XBullAdapter

```typescript
const wallet = new XBullAdapter();
```

### WalletConnectAdapter

```typescript
const wallet = new WalletConnectAdapter({
  projectId: "your_project_id",
  metadata: {
    name: "Your App",
    description: "App description",
    url: "https://yourapp.com",
    icons: ["https://yourapp.com/icon.png"],
  },
});
```

### createPreferredWalletAdapter()

```typescript
function createPreferredWalletAdapter(
  networkPassphrase: string
): WalletAdapter
```

Auto-detects and returns the best available wallet.

## Types

### StellarGrantsSDKConfig

```typescript
interface StellarGrantsSDKConfig {
  contractId: string;
  rpcUrl?: string;
  proxyUrl?: string;
  horizonUrl?: string;
  customHeaders?: Record<string, string>;
  networkPassphrase: string;
  signer?: StellarGrantsSigner;
  wallet?: WalletAdapter;
  defaultFee?: string;
}
```

### GrantCreateInput

```typescript
interface GrantCreateInput {
  owner: string;
  title: string;
  description: string;
  budget: bigint;
  deadline: bigint;
  milestoneCount: number;
}
```

### IpfsUploadConfig

```typescript
interface IpfsUploadConfig {
  pinataJwt?: string;
  pinataApiKey?: string;
  pinataSecretKey?: string;
  metadataSchema?: "grant" | "milestone";
  name?: string;
  skipSchemaValidation?: boolean;
}
```

## Errors

### StellarGrantsError

Base error class.

```typescript
class StellarGrantsError extends Error {
  code?: string;
  details?: any;
}
```

### SorobanRevertError

Contract revert error.

```typescript
class SorobanRevertError extends StellarGrantsError {
  code: string;
}
```

### TransactionTimeoutError

Transaction timeout error.

```typescript
class TransactionTimeoutError extends StellarGrantsError {
  hash: string;
  timeoutMs: number;
}
```

### TransactionFailedError

Transaction failure error.

```typescript
class TransactionFailedError extends StellarGrantsError {
  hash: string;
  errorResult?: any;
}
```

## Constants

### CONTRACT_INTERFACE_VERSION

Current SDK interface version.

```typescript
export const CONTRACT_INTERFACE_VERSION = 1;
```
