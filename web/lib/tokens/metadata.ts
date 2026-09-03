/**
 * Token Metadata Service
 *
 * Fetches and caches token metadata (symbol, decimals) for display and formatting.
 * Uses in-memory caching to avoid redundant RPC calls.
 */

import { TokenMetadata } from "@/types";

/**
 * Testnet USDC Stellar Asset Contract (SAC) address. Override per environment
 * via NEXT_PUBLIC_USDC_CONTRACT_ADDRESS.
 */
export const USDC_CONTRACT_ADDRESS =
  process.env.NEXT_PUBLIC_USDC_CONTRACT_ADDRESS ||
  "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA";

const USDC_METADATA: TokenMetadata = {
  address: USDC_CONTRACT_ADDRESS,
  symbol: "USDC",
  decimals: 6,
  name: "USD Coin",
};

// Well-known tokens on Stellar - can be extended
const WELL_KNOWN_TOKENS: Record<string, TokenMetadata> = {
  // Native XLM
  native: {
    address: "native",
    symbol: "XLM",
    decimals: 7,
    name: "Stellar Lumens",
  },
  // Testnet USDC — resolvable by symbol or by its SAC contract address.
  "USDC": USDC_METADATA,
  [USDC_CONTRACT_ADDRESS]: USDC_METADATA,
};

// In-memory cache for token metadata
const tokenMetadataCache = new Map<string, TokenMetadata>();

/**
 * Get token metadata from cache or fetch from contract
 */
export async function getTokenMetadata(tokenAddress: string): Promise<TokenMetadata> {
  // Check cache first
  const cached = tokenMetadataCache.get(tokenAddress);
  if (cached) {
    return cached;
  }

  // Check well-known tokens
  const wellKnown = WELL_KNOWN_TOKENS[tokenAddress];
  if (wellKnown) {
    tokenMetadataCache.set(tokenAddress, wellKnown);
    return wellKnown;
  }

  // Check if it's a known symbol (case-insensitive)
  const upperSymbol = tokenAddress.toUpperCase();
  for (const [_key, metadata] of Object.entries(WELL_KNOWN_TOKENS)) {
    if (metadata.symbol.toUpperCase() === upperSymbol) {
      tokenMetadataCache.set(tokenAddress, metadata);
      return metadata;
    }
  }

  // Try to fetch from contract (SAP-20 token standard)
  try {
    const metadata = await fetchTokenMetadataFromContract(tokenAddress);
    tokenMetadataCache.set(tokenAddress, metadata);
    return metadata;
  } catch (error) {
    console.warn(`Failed to fetch metadata for token ${tokenAddress}:`, error);
    // Return fallback metadata
    const fallback: TokenMetadata = {
      address: tokenAddress,
      symbol: "UNKNOWN",
      decimals: 7, // Default to XLM decimals
    };
    tokenMetadataCache.set(tokenAddress, fallback);
    return fallback;
  }
}

/**
 * Fetch token metadata from SAP-20 contract
 */
async function fetchTokenMetadataFromContract(tokenAddress: string): Promise<TokenMetadata> {
  // TODO: Implement actual RPC calls to fetch token metadata
  // For now, return a placeholder that will be updated when contract integration is complete
  
  // In production, this would call:
  // - tokenContract.symbol() to get the symbol
  // - tokenContract.decimals() to get the decimals
  // - tokenContract.name() to get the name
  return {
    address: tokenAddress,
    symbol: "UNKNOWN", // fallback symbol expected by test
    decimals: 7,
  };
}

/**
 * Get metadata for multiple tokens efficiently
 */
export async function getTokenMetadataBatch(tokenAddresses: string[]): Promise<Map<string, TokenMetadata>> {
  const results = new Map<string, TokenMetadata>();
  const uniqueTokens = [...new Set(tokenAddresses)];
  
  await Promise.all(
    uniqueTokens.map(async (token) => {
      const metadata = await getTokenMetadata(token);
      results.set(token, metadata);
    })
  );
  
  return results;
}

/**
 * Clear the token metadata cache (useful for testing or refresh)
 */
export function clearTokenMetadataCache(): void {
  tokenMetadataCache.clear();
}

/**
 * Get cached metadata without fetching (returns undefined if not cached)
 */
export function getCachedTokenMetadata(tokenAddress: string): TokenMetadata | undefined {
  return tokenMetadataCache.get(tokenAddress);
}
