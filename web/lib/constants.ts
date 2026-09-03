/**
 * Network & Contract Configuration
 *
 * Single source of truth for all environment-derived constants.
 * Import from here instead of reading process.env directly.
 */

export const STELLAR_NETWORK =
  (process.env.NEXT_PUBLIC_STELLAR_NETWORK as "testnet" | "mainnet") ?? "testnet";

export const CONTRACT_ID = process.env.NEXT_PUBLIC_CONTRACT_ID ?? "";

export const API_URL =
  process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000";

export const HORIZON_URL =
  process.env.NEXT_PUBLIC_HORIZON_URL ?? "https://horizon-testnet.stellar.org";

export const STELLAR_RPC_URL =
  process.env.NEXT_PUBLIC_STELLAR_RPC_URL ?? "https://soroban-testnet.stellar.org";

export const NETWORK_PASSPHRASE =
  process.env.NEXT_PUBLIC_NETWORK_PASSPHRASE ?? "Test SDF Network ; September 2015";

export const STELLAR_EXPLORER_BASE =
  STELLAR_NETWORK === "mainnet"
    ? "https://stellarchain.io"
    : "https://testnet.stellarchain.io";

export const GRANTS_PER_PAGE = 12;
export const LEADERBOARD_PAGE_SIZE = 25;

// Warn during development if the contract ID is missing.
if (process.env.NODE_ENV !== "production" && CONTRACT_ID === "") {
  console.warn(
    "[StellarGrant] NEXT_PUBLIC_CONTRACT_ID is not set. " +
      "Contract calls will fail. Copy .env.local.example to .env.local and fill in the values."
  );
}

/**
 * Returns the Stellar explorer URL for a transaction hash.
 *
 * @example
 * stellarExplorerTx("abc123...") // → "https://testnet.stellarchain.io/transactions/abc123..."
 */
export function stellarExplorerTx(txHash: string): string {
  return `${STELLAR_EXPLORER_BASE}/transactions/${txHash}`;
}

/**
 * Returns the Stellar explorer URL for an account address.
 */
export function stellarExplorerAccount(address: string): string {
  return `${STELLAR_EXPLORER_BASE}/accounts/${address}`;
}
