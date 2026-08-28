# Wallet Adapters

The StellarGrants SDK ships with built-in adapters for the most common Stellar wallets and exposes a clean interface for implementing your own.

---

## The `WalletAdapter` Interface

Every adapter must satisfy this interface:

```typescript
interface WalletAdapter {
  /** Human-readable wallet name, e.g. "Freighter". */
  readonly name: string;

  /** Optional URL to a wallet icon (SVG or PNG). Used by UI components. */
  readonly icon?: string;

  /**
   * Returns true if this wallet is available in the current environment.
   * Browser extension adapters check window globals; WalletConnect always returns true.
   * Must be synchronous.
   */
  isAvailable(): boolean;

  /** Returns the user's active Stellar public key (G... address). */
  getPublicKey(): Promise<string>;

  /**
   * Signs a transaction XDR and returns the signed XDR string.
   * @param txXdr            Base64-encoded transaction XDR.
   * @param networkPassphrase Stellar network passphrase (e.g. Networks.TESTNET).
   */
  signTransaction(txXdr: string, networkPassphrase: string): Promise<string>;

  /** Optional: initiate a pairing flow (WalletConnect). */
  connect?(networkPassphrase: string): Promise<{ uri: string; approval: () => Promise<void> }>;

  /** Optional: tear down the session. */
  disconnect?(): Promise<void>;

  /** True when a session is active (WalletConnect). */
  isConnected?: boolean;
}
```

---

## Built-in Adapters

| Adapter | Import | Mechanism |
|---|---|---|
| `FreighterAdapter` | `@stellargrants/client-sdk` | `@stellar/freighter-api` npm package |
| `AlbedoAdapter` | `@stellargrants/client-sdk` | `window.albedo` popup |
| `XBullAdapter` | `@stellargrants/client-sdk` | `window.xBull` extension |
| `WalletConnectAdapter` | `@stellargrants/client-sdk` | Pre-initialised `@walletconnect/sign-client` |

### Freighter

```typescript
import { FreighterAdapter, StellarGrantsSDK } from "@stellargrants/client-sdk";
import { Networks } from "@stellar/stellar-sdk";

const sdk = new StellarGrantsSDK({
  contractId: "C...",
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: Networks.TESTNET,
  wallet: new FreighterAdapter(),
});
```

### Albedo

```typescript
import { AlbedoAdapter, StellarGrantsSDK } from "@stellargrants/client-sdk";
import { Networks } from "@stellar/stellar-sdk";

const sdk = new StellarGrantsSDK({
  contractId: "C...",
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: Networks.TESTNET,
  wallet: new AlbedoAdapter(Networks.TESTNET),
});
```

### Automatic detection (Freighter → Albedo → xBull fallback)

```typescript
import { createPreferredWalletAdapter, StellarGrantsSDK } from "@stellargrants/client-sdk";
import { Networks } from "@stellar/stellar-sdk";

const sdk = new StellarGrantsSDK({
  contractId: "C...",
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: Networks.TESTNET,
  wallet: createPreferredWalletAdapter(Networks.TESTNET),
});
```

### WalletConnect

```typescript
import SignClient from "@walletconnect/sign-client";
import { WalletConnectAdapter, StellarGrantsSDK } from "@stellargrants/client-sdk";
import { Networks } from "@stellar/stellar-sdk";

const signClient = await SignClient.init({
  projectId: "YOUR_PROJECT_ID",
  metadata: { name: "My App", description: "...", url: "https://...", icons: [] },
});

const adapter = new WalletConnectAdapter(signClient);
const { uri, approval } = await adapter.connect(Networks.TESTNET);
// Display `uri` as a QR code, then:
await approval();

const sdk = new StellarGrantsSDK({
  contractId: "C...",
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: Networks.TESTNET,
  wallet: adapter,
});
```

---

## Implementing a Custom Adapter

Any object that satisfies `WalletAdapter` works. Here is a minimal example using a raw Stellar keypair (useful for scripts and tests):

```typescript
import { Keypair, TransactionBuilder } from "@stellar/stellar-sdk";
import type { WalletAdapter } from "@stellargrants/client-sdk";

export class KeypairAdapter implements WalletAdapter {
  readonly name = "Keypair";
  private readonly keypair: Keypair;

  constructor(secret: string) {
    this.keypair = Keypair.fromSecret(secret);
  }

  isAvailable(): boolean {
    return true; // always available — no extension required
  }

  async getPublicKey(): Promise<string> {
    return this.keypair.publicKey();
  }

  async signTransaction(txXdr: string, networkPassphrase: string): Promise<string> {
    const tx = TransactionBuilder.fromXDR(txXdr, networkPassphrase);
    tx.sign(this.keypair);
    return tx.toXDR();
  }
}

// Usage:
const sdk = new StellarGrantsSDK({
  contractId: "C...",
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: Networks.TESTNET,
  wallet: new KeypairAdapter(process.env.SECRET_KEY!),
});
```

### Checklist for custom adapters

- `name` — provide a human-readable string
- `isAvailable()` — return `true` if the wallet can be used right now
- `getPublicKey()` — return the active G... address; request permission if needed
- `signTransaction()` — validate the network passphrase before signing; return the signed XDR string
- `disconnect()` — optional; implement if your wallet has a session to tear down

---

## Using a Custom Adapter in the Frontend (`useWallet`)

The `useWallet` hook in `stellargrant-fe` supports `"freighter"` and `"albedo"` out of the box. To use a custom adapter, instantiate it directly and call `setActiveAdapter` from the wallet store:

```typescript
import { useWalletStore } from "@/lib/store/walletStore";
import { KeypairAdapter } from "./KeypairAdapter";

// In a component or effect:
const { setAddress, setWalletType, setActiveAdapter } = useWalletStore();

const adapter = new KeypairAdapter(secret);
const pubkey = await adapter.getPublicKey();

setAddress(pubkey);
setWalletType(null); // or add a custom type to the WalletType union
setActiveAdapter(adapter);
```

After this, `useWallet().signTransaction(xdr)` will delegate to your custom adapter automatically.
