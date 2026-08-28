# Design Document: wallet-adapter-support

## Overview

This document describes the design for adding first-class, multi-wallet support to the StellarGrants SDK and frontend. The work has two distinct layers:

1. **SDK layer** (`client/`) — harden the existing `WalletAdapter` interface, fix the `FreighterAdapter` to use the official `@stellar/freighter-api` npm package, add an `isAvailable()` detection method to all adapters, export everything from the public `index.ts`, and add a `wallet` convenience field to `StellarGrantsSDKConfig`.

2. **Frontend layer** (`stellargrant-fe/`) — wire the SDK adapters into the `useWallet` hook and `WalletSelectModal` so Albedo becomes a live option alongside Freighter, and the wallet store tracks the active adapter instance.

The goal is that switching wallet providers requires changing a single line of code, and a developer can implement a custom adapter by satisfying a well-documented interface.

---

## Architecture

```
client/src/
  types/index.ts                  ← extend WalletAdapter interface (isAvailable, name, icon)
  wallets/
    FreighterAdapter.ts           ← rewrite to use @stellar/freighter-api npm package
    AlbedoAdapter.ts              ← add isAvailable(), minor hardening
    XBullAdapter.ts               ← add isAvailable()
    WalletConnectAdapter.ts       ← add isAvailable() (always true — pairing-based)
    createPreferredWalletAdapter.ts ← update to use isAvailable()
    index.ts                      ← already exports all adapters (no change needed)
  index.ts                        ← add wallet adapter exports + WalletAdapter type export

stellargrant-fe/
  hooks/useWallet.ts              ← accept any WalletAdapter, remove Freighter-only hardcoding
  lib/store/walletStore.ts        ← add activeAdapter field
  components/wallet/
    WalletSelectModal.tsx         ← enable Albedo option, use adapter.isAvailable() for gating
```

---

## Components and Interfaces

### 1. Extended `WalletAdapter` Interface

**File:** `client/src/types/index.ts`

The existing `WalletAdapter` type is extended with three new optional-but-recommended members:

```typescript
export type WalletAdapter = StellarGrantsSigner & {
  /** Human-readable wallet name, e.g. "Freighter", "Albedo" */
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
```

`name` and `isAvailable()` are the only additions that affect all adapters. `icon` is optional and used only by the frontend modal.

`StellarGrantsSDKConfig` gains a `wallet` alias for `signer` to make intent clearer:

```typescript
export type StellarGrantsSDKConfig = {
  // ... existing fields ...
  signer?: StellarGrantsSigner;
  /** Alias for signer. If both are provided, wallet takes precedence. */
  wallet?: WalletAdapter;
};
```

---

### 2. `FreighterAdapter` — Rewrite

**File:** `client/src/wallets/FreighterAdapter.ts`

The current implementation uses `window.freighterApi` (the old global injection pattern). The `@stellar/freighter-api` npm package is already installed in `stellargrant-fe/` and must be added as a dependency to `client/package.json`.

New implementation uses the official package API:

```typescript
import {
  isConnected,
  isAllowed,
  setAllowed,
  getAddress,
  getNetworkDetails,
  signTransaction,
} from "@stellar/freighter-api";

export class FreighterAdapter implements WalletAdapter {
  readonly name = "Freighter";
  readonly icon = "https://freighter.app/favicon.ico";

  isAvailable(): boolean {
    // The npm package exposes window.freighter under the hood;
    // checking for the global is the most reliable sync detection.
    return typeof window !== "undefined" && Boolean((window as any).freighter);
  }

  async getPublicKey(): Promise<string> { ... }
  async signTransaction(txXdr: string, networkPassphrase: string): Promise<string> { ... }
}
```

Key changes from the current implementation:
- Uses named imports from `@stellar/freighter-api` instead of `window.freighterApi.*`
- `getPublicKey` calls `setAllowed()` then `getAddress()`, checking `.error` on the result object (v2 API pattern)
- `signTransaction` calls `getNetworkDetails()` to validate network, then `signTransaction()` from the package, checking `.error` on result
- No `Buffer.isBuffer` fallback needed — the v2 API always returns `{ signedTxXdr: string } | { error: string }`

---

### 3. `AlbedoAdapter` — Hardening

**File:** `client/src/wallets/AlbedoAdapter.ts`

The existing implementation is mostly correct. Changes:

```typescript
export class AlbedoAdapter implements WalletAdapter {
  readonly name = "Albedo";
  readonly icon = "https://albedo.link/img/albedo-logo.svg";

  isAvailable(): boolean {
    return typeof window !== "undefined" && Boolean((window as any).albedo);
  }

  // getPublicKey and signTransaction remain as-is (already correct)
}
```

---

### 4. `XBullAdapter` — Add `isAvailable`

**File:** `client/src/wallets/XBullAdapter.ts`

```typescript
export class XBullAdapter implements WalletAdapter {
  readonly name = "xBull";
  readonly icon = "https://xbull.app/assets/icons/icon-192x192.png";

  isAvailable(): boolean {
    return typeof window !== "undefined" && Boolean((window as any).xBull);
  }

  // getPublicKey and signTransaction remain as-is
}
```

---

### 5. `WalletConnectAdapter` — Add `isAvailable`

**File:** `client/src/wallets/WalletConnectAdapter.ts`

WalletConnect is pairing-based and always "available" (no extension to detect):

```typescript
export class WalletConnectAdapter implements WalletAdapter {
  readonly name = "WalletConnect";

  isAvailable(): boolean {
    return true; // pairing-based; no extension required
  }

  // existing connect/disconnect/getPublicKey/signTransaction unchanged
}
```

---

### 6. `createPreferredWalletAdapter` — Update

**File:** `client/src/wallets/createPreferredWalletAdapter.ts`

Replace the manual `window` checks with `isAvailable()`:

```typescript
export function createPreferredWalletAdapter(networkPassphrase?: string): WalletAdapter {
  const candidates: WalletAdapter[] = [
    new FreighterAdapter(),
    new AlbedoAdapter(networkPassphrase),
    new XBullAdapter(),
  ];
  const found = candidates.find((a) => a.isAvailable());
  if (!found) {
    throw new Error(
      "No supported wallet detected. Install Freighter, Albedo, or xBull and refresh.",
    );
  }
  return found;
}
```

---

### 7. SDK Public Exports

**File:** `client/src/index.ts`

Add wallet adapter exports so consumers can import directly from `@stellargrants/client-sdk`:

```typescript
// existing exports ...

// Wallet adapters
export { FreighterAdapter } from "./wallets/FreighterAdapter";
export { AlbedoAdapter } from "./wallets/AlbedoAdapter";
export { XBullAdapter } from "./wallets/XBullAdapter";
export { WalletConnectAdapter } from "./wallets/WalletConnectAdapter";
export { createPreferredWalletAdapter } from "./wallets/createPreferredWalletAdapter";
export type { WalletAdapter } from "./types";
```

---

### 8. `StellarGrantsSDK` — Accept `wallet` field

**File:** `client/src/StellarGrantsSDK.ts`

In the constructor, resolve `wallet` vs `signer`:

```typescript
constructor(config: StellarGrantsSDKConfig) {
  // wallet takes precedence over signer if both provided
  this.config = {
    ...config,
    signer: config.wallet ?? config.signer,
  };
  // ... rest of constructor unchanged
}
```

---

### 9. Frontend: `walletStore` — Track Active Adapter

**File:** `stellargrant-fe/lib/store/walletStore.ts`

Add `activeAdapter` to the store. Because Zustand's `persist` middleware cannot serialize class instances, the adapter is stored in memory only (not persisted):

```typescript
type WalletType = "freighter" | "albedo" | "xbull" | "walletconnect" | "passkey" | null;

interface WalletStore {
  // ... existing fields ...
  walletType: WalletType;                    // extend union to include "albedo"
  activeAdapter: WalletAdapter | null;       // in-memory only, not persisted
  setActiveAdapter: (adapter: WalletAdapter | null) => void;
}
```

The `persist` config excludes `activeAdapter`:

```typescript
persist(
  (set) => ({ ... }),
  {
    name: "stellar-grants-wallet",
    partialize: (state) => ({
      address: state.address,
      walletType: state.walletType,
      network: state.network,
    }),
  }
)
```

---

### 10. Frontend: `useWallet` Hook — Adapter-Agnostic

**File:** `stellargrant-fe/hooks/useWallet.ts`

The hook is refactored to delegate to the active `WalletAdapter` instead of hardcoding Freighter API calls:

```typescript
export interface WalletState {
  // ... existing fields ...
  walletType: "freighter" | "albedo" | "xbull" | "walletconnect" | "passkey" | null;
  connect: (type: "freighter" | "albedo" | "xbull" | "passkey") => Promise<void>;
  // signTransaction unchanged in signature
}
```

`connect("albedo")` instantiates `new AlbedoAdapter()`, calls `getPublicKey()` to get the address, stores the adapter in `walletStore.activeAdapter`.

`signTransaction` reads `activeAdapter` from the store and calls `adapter.signTransaction(xdr, networkPassphrase)` — no wallet-type branching needed.

Session restore on mount: if `walletType === "freighter"`, instantiate `FreighterAdapter` and call `getPublicKey()` to re-validate. If `walletType === "albedo"`, instantiate `AlbedoAdapter` — Albedo does not persist sessions, so no auto-restore (address is cleared).

---

### 11. Frontend: `WalletSelectModal` — Enable Albedo

**File:** `stellargrant-fe/components/wallet/WalletSelectModal.tsx`

- Import `AlbedoAdapter` (or use `isAvailable()` via a lightweight check) to gate the Albedo button
- Replace the hardcoded `connect("freighter")` call with a `connect(type)` call per option
- Albedo button is enabled when `AlbedoAdapter.isAvailable()` returns true; otherwise shows "Not installed" label
- xBull and Passkey remain "Coming soon" (out of scope for this feature)

```typescript
const walletOptions = [
  {
    id: "freighter",
    name: "Freighter",
    desc: "Stellar official browser extension",
    available: new FreighterAdapter().isAvailable(),
  },
  {
    id: "albedo",
    name: "Albedo",
    desc: "Web-based Stellar signer",
    available: new AlbedoAdapter().isAvailable(),
  },
];
```

---

## Data Models

No new database entities or API contracts are introduced. The only data model changes are the TypeScript type extensions described above.

---

## Error Handling

| Scenario | Behavior |
|---|---|
| `FreighterAdapter.getPublicKey()` — extension not installed | Throws `"Freighter extension is not installed"` |
| `FreighterAdapter.getPublicKey()` — permission denied | Throws `"Freighter permission denied"` |
| `FreighterAdapter.signTransaction()` — wrong network | Throws `"Freighter is on wrong network. Expected: <passphrase>"` |
| `AlbedoAdapter.getPublicKey()` — popup blocked | Throws `"Albedo popup was blocked or closed..."` (existing behavior, unchanged) |
| `AlbedoAdapter` — `window.albedo` absent | Throws `"Albedo is not installed or available"` (existing behavior) |
| `createPreferredWalletAdapter()` — no wallet found | Throws descriptive error listing all supported wallets |
| `StellarGrantsSDK` — neither `signer` nor `wallet` provided | Existing `"A signer is required..."` error unchanged |

---

## Custom Adapter Documentation

A `WALLET_ADAPTERS.md` file is added to `client/` documenting:

1. The `WalletAdapter` interface with JSDoc for each method
2. A minimal custom adapter example (e.g., a mock/test adapter)
3. How to pass a custom adapter to `StellarGrantsSDK`
4. How to pass a custom adapter to the frontend `useWallet` hook (via `connect` override or direct store injection)

---

## Testing Strategy

The existing Jest suite in `client/tests/` already covers `FreighterAdapter` and `AlbedoAdapter`. After this change:

- `FreighterAdapter` tests must be updated to mock `@stellar/freighter-api` named imports instead of `window.freighterApi`
- New tests for `isAvailable()` on all adapters (mock `window` globals)
- New test for `createPreferredWalletAdapter` using `isAvailable()` fallback chain
- New test for `StellarGrantsSDK` constructor: `wallet` field takes precedence over `signer`

No new test framework is needed — existing Jest + ts-jest setup is sufficient.

---

## Dependency Changes

| Package | Location | Change |
|---|---|---|
| `@stellar/freighter-api` | `client/package.json` | Add as dependency (already in `stellargrant-fe`) |

No other new dependencies. Albedo and xBull adapters continue to use `window` globals (no npm package needed — Albedo's npm package is optional and the `window.albedo` injection is the standard integration path).
