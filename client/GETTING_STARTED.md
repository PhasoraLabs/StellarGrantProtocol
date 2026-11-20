# Getting Started with StellarGrants SDK

This guide will help you get up and running with the StellarGrants SDK in minutes.

## Prerequisites

- Node.js 18+ installed
- A Stellar wallet (Freighter, Albedo, or xBull)
- Access to a Stellar testnet or mainnet
- Basic knowledge of TypeScript/JavaScript

## Installation

```bash
npm install @stellargrants/client-sdk
```

## Quick Start

### 1. Set Up Environment Variables

Create a `.env` file in your project root:

```env
# Stellar Network Configuration
RPC_URL=https://soroban-testnet.stellar.org
NETWORK_PASSPHRASE=Test SDF Network ; September 2015
CONTRACT_ID=your_contract_id_here

# Optional: For IPFS integration
PINATA_JWT=your_pinata_jwt_here

# Optional: For dynamic fee estimation
HORIZON_URL=https://horizon-testnet.stellar.org
```

### 2. Initialize the SDK

```typescript
import { StellarGrantsSDK, FreighterAdapter } from "@stellargrants/client-sdk";

// Use Freighter wallet
const wallet = new FreighterAdapter();

if (!wallet.isAvailable()) {
  throw new Error("Please install Freighter wallet");
}

const sdk = new StellarGrantsSDK({
  contractId: process.env.CONTRACT_ID!,
  rpcUrl: process.env.RPC_URL!,
  networkPassphrase: process.env.NETWORK_PASSPHRASE!,
  wallet,
  horizonUrl: process.env.HORIZON_URL, // Optional
});
```

### 3. Create Your First Grant

```typescript
async function createGrant() {
  const owner = await wallet.getPublicKey();

  const result = await sdk.grantCreate({
    owner,
    title: "My First Grant",
    description: "Building something amazing on Stellar",
    budget: BigInt("1000000000"), // 1000 XLM in stroops
    deadline: BigInt(Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 30), // 30 days
    milestoneCount: 3,
  });

  console.log("Grant created:", result);
}
```

### 4. Fund a Grant

```typescript
async function fundGrant(grantId: number) {
  const result = await sdk.grantFund({
    grantId,
    token: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC", // USDC
    amount: BigInt("500000000"), // 500 USDC
  });

  console.log("Grant funded:", result);
}
```

### 5. Read Grant Data

```typescript
async function getGrant(grantId: number) {
  const grant = await sdk.grantGet(grantId);
  console.log("Grant details:", grant);
}
```

## Common Use Cases

### Working with Milestones

```typescript
// Submit milestone proof
await sdk.milestoneSubmit({
  grantId: 1,
  milestoneIdx: 0,
  proofHash: "QmX7Y8Z...", // IPFS CID
});

// Vote on milestone
await sdk.milestoneVote({
  grantId: 1,
  milestoneIdx: 0,
  approve: true,
});

// Get milestone details
const milestone = await sdk.milestoneGet(1, 0);
```

### Using IPFS for Metadata

```typescript
import { uploadMetadataToIPFS } from "@stellargrants/client-sdk";

// Upload rich metadata
const { cid } = await uploadMetadataToIPFS(
  {
    title: "Detailed Grant Info",
    description: "Long description...",
    team: ["Alice", "Bob"],
    attachments: ["https://example.com/doc.pdf"],
  },
  {
    pinataJwt: process.env.PINATA_JWT!,
  }
);

// Use CID in grant creation
await sdk.grantCreate({
  // ...other fields
  description: `ipfs://${cid}`,
});
```

### Implementing Optimistic UI

```typescript
import { TransactionTracker } from "@stellargrants/client-sdk";

const tracker = new TransactionTracker();

// Show immediate feedback
tracker.on("signed", () => {
  showNotification("Transaction signed");
});

tracker.on("submitted", (txId, hash) => {
  showNotification("Transaction submitted");
});

tracker.on("confirmed", (txId, result) => {
  showNotification("Success!");
  updateUI(result);
});

tracker.on("failed", (txId, error) => {
  showError(error.message);
});

// Track transaction
await tracker.track(async () => {
  return await sdk.grantCreate(input);
});
```

### Handling Errors

```typescript
import {
  StellarGrantsError,
  SorobanRevertError,
  TransactionTimeoutError,
} from "@stellargrants/client-sdk";

try {
  await sdk.grantCreate(input);
} catch (error) {
  if (error instanceof SorobanRevertError) {
    console.error("Contract error:", error.code);
  } else if (error instanceof TransactionTimeoutError) {
    console.error("Transaction timed out");
  } else if (error instanceof StellarGrantsError) {
    console.error("SDK error:", error.message);
  }
}
```

### Checking Compatibility

```typescript
const compatibility = await sdk.checkCompatibility();

if (!compatibility.compatible) {
  console.warn("Version mismatch:", compatibility.warning);
  // Prompt user to upgrade
}
```

## Best Practices

### 1. Always Handle Errors

```typescript
try {
  await sdk.grantCreate(input);
} catch (error) {
  // Handle error appropriately
  console.error(error);
  showUserFriendlyError(error);
}
```

### 2. Use Optimistic Updates for Better UX

```typescript
// Show immediate feedback
updateUIOptimistically(predictedState);

try {
  const result = await sdk.grantCreate(input);
  updateUIWithActualResult(result);
} catch (error) {
  rollbackOptimisticUpdate();
}
```

### 3. Store Large Data on IPFS

```typescript
// Don't store large descriptions on-chain
const { cid } = await uploadMetadataToIPFS(largeMetadata, config);

// Store only the CID reference
await sdk.grantCreate({
  // ...
  description: `ipfs://${cid}`,
});
```

### 4. Monitor Transaction Status

```typescript
const result = await sdk.waitForTransaction(txHash, {
  pollIntervalMs: 3000,
  timeoutMs: 60000,
  onStatusChange: (status) => {
    updateProgressBar(status);
  },
});
```

### 5. Use Dynamic Fee Estimation

```typescript
const sdk = new StellarGrantsSDK({
  // ...
  horizonUrl: "https://horizon-testnet.stellar.org",
});

// SDK will automatically use network fee data
await sdk.grantCreate(input, {
  feePriority: "medium", // or "low" / "high"
});
```

## Next Steps

- Explore the [full API documentation](./README.md#api-reference)
- Check out [complete examples](./examples/)
- Learn about [wallet integration](./README.md#wallet-adapters)
- Read about [error handling](./README.md#error-handling)

## Troubleshooting

### "Freighter wallet not found"

Install the Freighter browser extension from [freighter.app](https://www.freighter.app/).

### "Transaction timeout"

Increase the timeout or check network status:

```typescript
await sdk.waitForTransaction(hash, {
  timeoutMs: 120000, // 2 minutes
});
```

### "IPFS upload failed"

Verify your Pinata JWT is correct and has the necessary permissions.

### "Contract version mismatch"

Run compatibility check and upgrade SDK or contract:

```typescript
const compat = await sdk.checkCompatibility();
console.log(compat.warning);
```

## Support

- GitHub Issues: [StellarGrant/StellarGrant-fe/issues](https://github.com/StellarGrant/StellarGrant-fe/issues)
- Documentation: [README.md](./README.md)
- Examples: [examples/](./examples/)
