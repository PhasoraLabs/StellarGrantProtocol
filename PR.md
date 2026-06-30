## Summary

This PR implements four SDK improvements across a single branch to keep the changes cohesive and easy to review.

---

## Changes

### #498 — TypeScript Type Safety for Contract Read Methods

**Files:** `client/src/types/index.ts`, `client/src/index.ts`, `client/src/StellarGrantsSDK.ts`

- Added fully-typed `GrantData` and `MilestoneData` interfaces whose field names match Soroban's `scValToNative` output (snake_case keys exactly as returned by the contract).
- Updated `grantGet()` return type from `any` to `GrantData | null` and `milestoneGet()` to `MilestoneData | null`.
- Added `assertGrantData` / `assertMilestoneData` internal shape validators that emit a `console.warn` in non-production environments when the response is missing expected fields, without ever hard-throwing on partial data (safe for unit-test mocks).
- Exported new types from `src/index.ts`.

---

### #501 — Polling vs WebSockets for Contract Events

**Files:** `client/src/StellarGrantsSDK.ts`

- Added automatic WebSocket URL derivation from the configured `rpcUrl`: `http://` → `ws://`, `https://` → `wss://` — no separate `websocketUrl` config needed.
- Added a **5-second connection timeout** on WebSocket initiation; if the socket does not reach `onopen` within that window, the SDK falls back to HTTP polling.
- Set `useWebSocket = false` permanently after any connection error or close, preventing reconnect loops that keep hammering a non-WS RPC endpoint.
- WebSocket errors are forwarded to `options.onError` before cleanup.
- All existing WebSocket unit tests continue to pass.

---

### #503 — Dynamic Gas/Fee Estimation Improvements

**Files:** `client/src/StellarGrantsSDK.ts`

- Added a `private getDynamicFeeStats(horizonUrl)` helper that fetches `/fee_stats` from Horizon and selects the fee percentile adaptively based on real-time ledger saturation:

  | Network Load | Capacity Usage | Percentile | Modifiers (low / med / high) |
  |---|---|---|---|
  | `surge` | > 95% | p90 | 1.6 / 2.5 / 3.5 |
  | `high` | > 80% | p80 | 1.3 / 2.0 / 2.8 |
  | `moderate` | > 50% | p70 | 1.0 / 1.5 / 2.0 |
  | `normal` | ≤ 50% | p50 | 0.9 / 1.2 / 1.6 |

- `estimateFees()` now delegates to `getDynamicFeeStats()` and exposes `percentile` in the response alongside `networkLoad`, `source`, and fee tiers.
- `invokeWrite()` now queries Horizon (when `horizonUrl` is configured) to scale `minResourceFee` up to the competitive `recommendedBase` **before** applying the priority multiplier — preventing transactions from hanging during congestion while avoiding overpayment during idle periods.
- Falls back gracefully to `simulation-fallback` static tiers when Horizon is unavailable.

---

### #497 — SDK Bundle Size Optimization

**Files:** `client/tsup.config.ts` *(new)*, `client/package.json`

- Migrated the build pipeline from `tsc` to [`tsup`](https://tsup.egoist.dev/).
- New `tsup.config.ts` builds **dual CJS + ESM** output with tree-shaking, source maps, and proper externals — reducing the installed bundle footprint for downstream consumers.
- Added `module` and `exports` fields to `package.json` for ESM-first consumers and bundlers.
- Removed the unused `axios` runtime dependency (it was not imported anywhere in the codebase).

---

## Test Results

```
Test Suites: 20 passed, 20 total
Tests:       261 passed, 261 total
```

All existing tests pass. The fee estimation suite (`tests/fee-estimation.test.ts`) was verified to be unaffected by the dynamic stats refactor.

---

## Checklist

- [x] All 261 tests pass locally
- [x] No new lint or TypeScript errors
- [x] Backward compatible — existing callers need no config changes
- [x] `axios` removed from `dependencies`
- [x] `tsup` added to `devDependencies` only (not bundled into the SDK)

---

Closes #497
Closes #498
Closes #501
Closes #503
