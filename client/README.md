# StellarGrants Client SDK

TypeScript SDK for interacting with the StellarGrants Soroban contract via RPC simulation + transaction submission.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Core Features](#core-features)
  - [Grant Operations](#grant-operations)
  - [Milestone Operations](#milestone-operations)
  - [IPFS Integration](#ipfs-integration)
  - [Optimistic UI Updates](#optimistic-ui-updates)
  - [Transaction Tracking](#transaction-tracking)
  - [Compatibility Checks](#compatibility-checks)
- [Wallet Adapters](#wallet-adapters)
- [Error Handling](#error-handling)
- [Examples](#examples)
- [CLI](#cli)
- [API Reference](#api-reference)

## Installation

```bash
npm install @stellargrants/client-sdk
```

## Quick Start

### Create an SDK instance

```ts
import { StellarGrantsSDK } from "@stellargrants/client-sdk";

const sdk = new StellarGrantsSDK({
  contractId: process.env.CONTRACT_ID!,
  rpcUrl: process.env.RPC_URL!,
  networkPassphrase: process.env.NETWORK_PASSPHRASE!,
  signer: {
    async getPublicKey() {
      // return active wallet public key
      return "G...";
    },
    async signTransaction(txXdr, networkPassphrase) {
      // sign using a wallet (Freighter / Albedo / custom signer)
      return txXdr;
    },
  },
});
```

### Create a grant

```ts
const result = await sdk.grantCreate({
  owner: await signer.getPublicKey(),
  title: "Ocean Cleanup Initiative",
  description: "Funding for coastal cleanup operations",
  budget: BigInt("5000000000"), // 5000 XLM in stroops
  deadline: BigInt(Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 90), // 90 days
  milestoneCount: 4,
});
```

## Configuration

### `StellarGrantsSDKConfig`

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `contractId` | `string` | Yes | Soroban contract ID (hex string) |
| `rpcUrl` | `string` | Yes* | Soroban RPC endpoint URL |
| `proxyUrl` | `string` | Yes* | Alternative proxy URL (if not using rpcUrl) |
| `networkPassphrase` | `string` | Yes | Network passphrase (Testnet/Mainnet) |
| `signer` | `StellarGrantsSigner` | Yes | Transaction signer implementation |
| `wallet` | `WalletAdapter` | No | Alias for signer (takes precedence) |
| `horizonUrl` | `string` | No | Horizon endpoint for dynamic fee estimation |
| `customHeaders` | `Record<string, string>` | No | Custom HTTP headers for RPC requests |
| `defaultFee` | `string` | No | Default fee in stroops (default: "100") |

*Either `rpcUrl` or `proxyUrl` must be provided.

### `StellarGrantsSigner`

Interface for transaction signing:

```ts
interface StellarGrantsSigner {
  getPublicKey(): Promise<string>;
  signTransaction(txXdr: string, networkPassphrase: string): Promise<string>;
}
```

## Core Features

### Grant Operations

#### Create a Grant

```ts
const result = await sdk.grantCreate({
  owner: "GABC...",
  title: "Educational Program",
  description: "STEM education for underserved communities",
  budget: BigInt("10000000000"),
  deadline: BigInt(Date.now() / 1000 + 86400 * 60),
  milestoneCount: 5,
});
```

#### Fund a Grant

```ts
await sdk.grantFund({
  grantId: 1,
  token: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC", // USDC
  amount: BigInt("1000000000"), // 1000 USDC
});
```

#### Get Grant Details

```ts
const grant = await sdk.grantGet(1);
console.log(grant);
```

### Milestone Operations

#### Submit Milestone Proof

```ts
await sdk.milestoneSubmit({
  grantId: 1,
  milestoneIdx: 0,
  proofHash: "QmX7Y8Z...", // IPFS CID or hash
});
```

#### Vote on Milestone

```ts
await sdk.milestoneVote({
  grantId: 1,
  milestoneIdx: 0,
  approve: true,
});
```

#### Get Milestone Details

```ts
const milestone = await sdk.milestoneGet(1, 0);
console.log(milestone);
```

### IPFS Integration

Store large grant metadata off-chain to reduce on-chain storage costs.

#### Upload Metadata to IPFS

```ts
import { uploadMetadataToIPFS } from "@stellargrants/client-sdk";

const { cid, gatewayUrl } = await uploadMetadataToIPFS(
  {
    title: "Ocean Cleanup Initiative",
    description: "Detailed project description...",
    attachments: ["https://example.com/doc.pdf"],
    team: ["Alice", "Bob"],
  },
  {
    pinataJwt: process.env.PINATA_JWT,
  }
);

console.log(`Metadata stored at: ipfs://${cid}`);
```

#### Fetch Metadata from IPFS

```ts
import { fetchMetadataFromIPFS } from "@stellargrants/client-sdk";

const metadata = await fetchMetadataFromIPFS("QmX7Y8Z...");
console.log(metadata);
```

#### Automatic IPFS Integration

```ts
// Upload metadata automatically during grant creation
const result = await sdk.grantCreate(
  {
    owner: "GABC...",
    title: "My Grant",
    description: "This will be uploaded to IPFS",
    budget: BigInt(1000000),
    deadline: BigInt(Date.now() / 1000 + 86400 * 30),
    milestoneCount: 3,
  },
  {
    uploadMetadata: true,
    ipfsConfig: { pinataJwt: process.env.PINATA_JWT },
  }
);

// Fetch metadata automatically when reading grant
const grant = await sdk.grantGet(1, {
  fetchIpfsMetadata: true,
  ipfsGateways: ["https://gateway.pinata.cloud/ipfs/"],
});
```

### Optimistic UI Updates

Provide instant feedback to users while transactions are being processed.

#### Transaction Tracker

```ts
import { TransactionTracker } from "@stellargrants/client-sdk";

const tracker = new TransactionTracker();

// Listen to transaction stages
tracker.on("signed", (txId) => {
  console.log("Transaction signed:", txId);
  showNotification("Transaction signed, submitting...");
});

tracker.on("submitted", (txId, hash) => {
  console.log("Transaction submitted:", hash);
  showNotification("Transaction submitted, waiting for confirmation...");
});

tracker.on("confirmed", (txId, result) => {
  console.log("Transaction confirmed:", result);
  showNotification("Transaction confirmed!");
  updateUI(result);
});

tracker.on("failed", (txId, error) => {
  console.error("Transaction failed:", error);
  showError(error.message);
});

// Track a transaction
const txId = await tracker.track(async () => {
  return await sdk.grantCreate(input);
});
```

#### Optimistic State Manager

```ts
import { OptimisticStateManager } from "@stellargrants/client-sdk";

const manager = new OptimisticStateManager();

// Predict state after grant creation
const predictedGrant = manager.predictGrantCreate({
  owner: "GABC...",
  title: "New Grant",
  description: "Description",
  budget: BigInt(1000000),
  deadline: BigInt(Date.now() / 1000 + 86400 * 30),
  milestoneCount: 3,
});

// Apply optimistic update
manager.apply("tx_123", predictedGrant, "grant_create");

// Update UI immediately
updateGrantsList([...existingGrants, predictedGrant]);

try {
  // Execute actual transaction
  const result = await sdk.grantCreate(input);
  
  // Commit on success
  manager.commit("tx_123", result);
  updateGrantsList([...existingGrants, result]);
} catch (error) {
  // Rollback on failure
  const previousState = manager.rollback("tx_123");
  updateGrantsList(existingGrants);
  showError("Transaction failed");
}
```

### Transaction Tracking

Monitor transaction status with automatic polling and retries.

```ts
const result = await sdk.waitForTransaction(txHash, {
  pollIntervalMs: 3000,
  timeoutMs: 60000,
  maxNetworkRetries: 3,
  onStatusChange: (status) => {
    console.log("Status:", status);
  },
  onPoll: (attempt, elapsedMs) => {
    console.log(`Poll attempt ${attempt} (${elapsedMs}ms elapsed)`);
  },
});
```

### Compatibility Checks

Ensure SDK and contract versions are compatible.

```ts
const compatibility = await sdk.checkCompatibility();

if (!compatibility.compatible) {
  console.warn(compatibility.warning);
  // Prompt user to upgrade SDK or contract
}

console.log(`SDK version: ${compatibility.sdkVersion}`);
console.log(`Contract version: ${compatibility.contractVersion}`);
```

## Wallet Adapters

Built-in wallet adapters for popular Stellar wallets.

### Available Adapters

- `FreighterAdapter` - Freighter browser extension
- `AlbedoAdapter` - Albedo web wallet
- `XBullAdapter` - xBull wallet
- `WalletConnectAdapter` - WalletConnect protocol

### Using Wallet Adapters

```ts
import { FreighterAdapter, StellarGrantsSDK } from "@stellargrants/client-sdk";

const wallet = new FreighterAdapter();

if (!wallet.isAvailable()) {
  throw new Error("Freighter wallet not installed");
}

const sdk = new StellarGrantsSDK({
  contractId: process.env.CONTRACT_ID!,
  rpcUrl: process.env.RPC_URL!,
  networkPassphrase: "Test SDF Network ; September 2015",
  wallet, // Use wallet instead of signer
});
```

### Automatic Wallet Detection

```ts
import { createPreferredWalletAdapter } from "@stellargrants/client-sdk";

const networkPassphrase = "Test SDF Network ; September 2015";
const wallet = createPreferredWalletAdapter(networkPassphrase);

const sdk = new StellarGrantsSDK({
  contractId: process.env.CONTRACT_ID!,
  rpcUrl: process.env.RPC_URL!,
  networkPassphrase,
  wallet,
});
```

## Error Handling

The SDK provides typed errors for better error handling.

```ts
import {
  StellarGrantsError,
  SorobanRevertError,
  TransactionTimeoutError,
  TransactionFailedError,
} from "@stellargrants/client-sdk";

try {
  await sdk.grantCreate(input);
} catch (error) {
  if (error instanceof SorobanRevertError) {
    console.error("Contract reverted:", error.code, error.message);
  } else if (error instanceof TransactionTimeoutError) {
    console.error("Transaction timed out:", error.hash);
  } else if (error instanceof TransactionFailedError) {
    console.error("Transaction failed:", error.hash, error.errorResult);
  } else if (error instanceof StellarGrantsError) {
    console.error("SDK error:", error.code, error.message);
  }
}
```

## Examples

See the `examples/` directory for complete working examples:

- [`create-grant.ts`](./examples/create-grant.ts) - Create a new grant
- [`vote-on-milestone.ts`](./examples/vote-on-milestone.ts) - Vote on milestone approval
- [`ipfs-metadata.ts`](./examples/ipfs-metadata.ts) - Upload and fetch IPFS metadata
- [`optimistic-ui.ts`](./examples/optimistic-ui.ts) - Implement optimistic UI updates

## CLI

The SDK includes a command-line interface for common operations.

```bash
# Initialize configuration
npx @stellargrants/client-sdk init

# Check grant status
npx @stellargrants/client-sdk grant-status 1 --format json

# Fund a grant
npx @stellargrants/client-sdk fund-grant 1 --token CTOKEN... --amount 1000000

# Check compatibility
npx @stellargrants/client-sdk check-compatibility
```

## API Reference

### StellarGrantsSDK

#### Constructor

```ts
new StellarGrantsSDK(config: StellarGrantsSDKConfig)
```

#### Methods

##### Grant Operations

- `grantCreate(input: GrantCreateInput, options?): Promise<unknown>`
- `grantFund(input: GrantFundInput, options?): Promise<unknown>`
- `grantGet(grantId: number, options?): Promise<unknown>`

##### Milestone Operations

- `milestoneSubmit(input: MilestoneSubmitInput, options?): Promise<unknown>`
- `milestoneVote(input: MilestoneVoteInput, options?): Promise<unknown>`
- `milestoneGet(grantId: number, milestoneIdx: number): Promise<unknown>`

##### IPFS Operations

- `uploadMetadataToIPFS(metadata: Record<string, unknown>, config: IpfsUploadConfig): Promise<IpfsUploadResult>`
- `fetchMetadataFromIPFS(cid: string, gateways?: string[]): Promise<Record<string, unknown>>`

##### Transaction Management

- `waitForTransaction(hash: string, options?: WaitForTransactionOptions): Promise<TransactionResult>`
- `estimateFees(method: string, args: xdr.ScVal[], options?): Promise<any>`

##### Utilities

- `checkCompatibility(): Promise<{ compatible: boolean; sdkVersion: number; contractVersion: number | null; warning?: string }>`
- `subscribeToEvents(callback: (event: any) => void, options?): () => void`

### Standalone Functions

#### IPFS

```ts
uploadMetadataToIPFS(
  metadata: Record<string, unknown>,
  config: IpfsUploadConfig
): Promise<IpfsUploadResult>

fetchMetadataFromIPFS(
  cid: string,
  gateways?: string[]
): Promise<Record<string, unknown>>
```

### Classes

#### TransactionTracker

Event-driven transaction lifecycle tracking.

```ts
const tracker = new TransactionTracker();
tracker.on(event, listener);
tracker.track(txFn, predictedState);
```

#### OptimisticStateManager

Manage optimistic UI updates with rollback support.

```ts
const manager = new OptimisticStateManager();
manager.predictGrantCreate(input);
manager.apply(txId, predictedState, type);
manager.commit(txId, actualState);
manager.rollback(txId);
```

## License

MIT