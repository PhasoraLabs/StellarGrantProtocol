# Implementation Plan: wallet-adapter-support

## Overview

Harden the existing wallet adapter layer in the SDK, fix `FreighterAdapter` to use the official npm package, expose all adapters from the public SDK entry point, and wire Albedo as a live option in the frontend wallet modal.

## Tasks

- [x] 1. Extend `WalletAdapter` interface and `StellarGrantsSDKConfig`
  - Open `client/src/types/index.ts`
  - Add `readonly name: string` to `WalletAdapter`
  - Add `readonly icon?: string` to `WalletAdapter`
  - Add `isAvailable(): boolean` method to `WalletAdapter`
  - Add `wallet?: WalletAdapter` field to `StellarGrantsSDKConfig` as an alias for `signer`
  - Export `WalletAdapter` type from `client/src/index.ts`
  - _Requirements: Define a standard WalletAdapter interface_

- [x] 2. Add `@stellar/freighter-api` to SDK dependencies
  - Open `client/package.json`
  - Add `"@stellar/freighter-api": "^6.0.1"` to `dependencies`
  - Run `npm install` inside `client/` to update `package-lock.json`
  - _Requirements: Implement specific adapter for Freighter_

- [x] 3. Rewrite `FreighterAdapter` to use the official npm package
  - Open `client/src/wallets/FreighterAdapter.ts`
  - Replace all `window.freighterApi.*` calls with named imports from `@stellar/freighter-api`
  - Implement `isAvailable()`: return `typeof window !== "undefined" && Boolean((window as any).freighter)`
  - Add `readonly name = "Freighter"` and `readonly icon = "https://freighter.app/favicon.ico"`
  - Rewrite `getPublicKey()`: call `setAllowed()`, check `.isAllowed`, then call `getAddress()`, check `.error`, return `.address`
  - Rewrite `signTransaction()`: call `getNetworkDetails()` to validate network passphrase match, then call `signTransaction()` from the package, check `.error`, return `.signedTxXdr`
  - Update existing `FreighterAdapter` tests in `client/tests/wallets/freighter.test.ts` to mock `@stellar/freighter-api` named imports instead of `window.freighterApi`
  - _Requirements: Implement specific adapter for Freighter_

- [x] 4. Harden `AlbedoAdapter` — add interface members
  - Open `client/src/wallets/AlbedoAdapter.ts`
  - Add `readonly name = "Albedo"` and `readonly icon = "https://albedo.link/img/albedo-logo.svg"`
  - Add `isAvailable(): boolean` returning `typeof window !== "undefined" && Boolean((window as any).albedo)`
  - No changes to `getPublicKey` or `signTransaction` logic
  - _Requirements: Implement specific adapter for Albedo_

- [x] 5. Add `isAvailable` and metadata to `XBullAdapter`
  - Open `client/src/wallets/XBullAdapter.ts`
  - Add `readonly name = "xBull"` and `readonly icon = "https://xbull.app/assets/icons/icon-192x192.png"`
  - Add `isAvailable(): boolean` returning `typeof window !== "undefined" && Boolean((window as any).xBull)`
  - _Requirements: Define a standard WalletAdapter interface_

- [x] 6. Add `isAvailable` and metadata to `WalletConnectAdapter`
  - Open `client/src/wallets/WalletConnectAdapter.ts`
  - Add `readonly name = "WalletConnect"`
  - Add `isAvailable(): boolean` returning `true` (pairing-based, no extension required)
  - _Requirements: Define a standard WalletAdapter interface_

- [x] 7. Update `createPreferredWalletAdapter` to use `isAvailable()`
  - Open `client/src/wallets/createPreferredWalletAdapter.ts`
  - Replace manual `window.freighterApi` / `window.albedo` checks with `adapter.isAvailable()` calls
  - Build a candidates array: `[new FreighterAdapter(), new AlbedoAdapter(networkPassphrase), new XBullAdapter()]`
  - Return the first adapter where `isAvailable()` is true
  - Update error message to list all supported wallets by name
  - _Requirements: Switching between wallet providers requires minimal code changes_

- [x] 8. Export wallet adapters from SDK public entry point
  - Open `client/src/index.ts`
  - Add named exports for `FreighterAdapter`, `AlbedoAdapter`, `XBullAdapter`, `WalletConnectAdapter`
  - Add named export for `createPreferredWalletAdapter`
  - Add type export for `WalletAdapter`
  - _Requirements: Update StellarGrantsSDK to easily accept these adapters during configuration_

- [x] 9. Update `StellarGrantsSDK` constructor to resolve `wallet` vs `signer`
  - Open `client/src/StellarGrantsSDK.ts`
  - In the constructor, set `this.config.signer = config.wallet ?? config.signer` so `wallet` takes precedence
  - No other changes to the class
  - _Requirements: Update StellarGrantsSDK to easily accept these adapters during configuration_

- [x] 10. Checkpoint — build and test the SDK
  - Run `npm run build` inside `client/` and confirm zero TypeScript errors in our files
  - Run `npm test` inside `client/` and confirm all 146 tests pass
  - _Requirements: At least Freighter and Albedo are supported out of the box_

- [x] 11. Extend `walletStore` to track active adapter
  - Open `stellargrant-fe/lib/store/walletStore.ts`
  - Add `"albedo"` to the `WalletType` union
  - Add `activeAdapter: WalletAdapter | null` field (in-memory, not persisted)
  - Add `setActiveAdapter: (adapter: WalletAdapter | null) => void` action
  - Add `partialize` to the `persist` config to exclude `activeAdapter` from localStorage
  - Create `stellargrant-fe/lib/wallets/types.ts` with the frontend `WalletAdapter` interface
  - _Requirements: Switching between wallet providers requires minimal code changes_

- [x] 12. Refactor `useWallet` hook to be adapter-agnostic
  - Open `stellargrant-fe/hooks/useWallet.ts`
  - Add `"albedo"` to the `connect` type parameter union
  - In `connect("freighter")`: instantiate `FreighterAdapter`, call `getPublicKey()`, store adapter via `setActiveAdapter`
  - In `connect("albedo")`: instantiate `AlbedoAdapter`, call `getPublicKey()` (triggers popup), store adapter
  - Rewrite `signTransaction`: delegate to `activeAdapter.signTransaction()` — no wallet-type branching
  - Update session restore: re-instantiate `FreighterAdapter` for Freighter; clear address for Albedo (no persistent session)
  - Update `disconnect`: call `activeAdapter?.disconnect?.()` before `reset()`
  - Create `stellargrant-fe/lib/wallets/FreighterAdapter.ts` and `AlbedoAdapter.ts`
  - _Requirements: Switching between wallet providers requires minimal code changes_

- [x] 13. Enable Albedo in `WalletSelectModal`
  - Open `stellargrant-fe/components/wallet/WalletSelectModal.tsx`
  - Add `AlbedoIcon` SVG component
  - Build `walletOptions` array using `adapter.isAvailable()` to gate each option
  - Render active/unavailable state per option (not just "Coming soon")
  - Replace hardcoded `"Connecting to Freighter…"` with dynamic `"Connecting to {selectedWallet}…"`
  - Add `type="button"` to all `<button>` elements
  - Keep xBull and Passkey as "Coming soon"
  - _Requirements: At least Freighter and Albedo are supported out of the box_

- [x] 14. Write `WALLET_ADAPTERS.md` documentation
  - Create `client/WALLET_ADAPTERS.md`
  - Document the full `WalletAdapter` interface with description of each method and property
  - Provide a minimal custom adapter example (KeypairAdapter)
  - Show how to pass a custom adapter to `StellarGrantsSDK` via `wallet` field
  - Show how to use `createPreferredWalletAdapter()` for automatic detection
  - Show how to use a custom adapter with the frontend `useWallet` hook via `setActiveAdapter`
  - _Requirements: Clear documentation on how to implement a custom wallet adapter_

- [x] 15. Final checkpoint — full build and test
  - Run `npm test` inside `client/` — 146 tests pass
  - Run `npx tsc --noEmit` inside `stellargrant-fe/` — zero errors in our new files
  - _Requirements: All acceptance criteria met_
