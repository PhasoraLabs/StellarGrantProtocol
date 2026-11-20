import { inject, provide } from "vue";
import { StellarGrantsSDK } from "../StellarGrantsSDK";
import type { StellarGrantsSDKConfig } from "../types";
import { StellarGrantsContext, STELLAR_GRANTS_KEY } from "./types";

/**
 * Provides the StellarGrants SDK context to the Vue component tree.
 * This should be called in your app's root component or a specific subtree.
 *
 * @param config - Configuration for the StellarGrantsSDK
 * @param logger - Optional logger instance for debugging
 *
 * @example
 * ```vue
 * <script setup>
 * import { provideStellarGrants } from '@stellargrants/client-sdk';
 * 
 * provideStellarGrants({
 *   contractId: 'CD...',
 *   rpcUrl: 'https://soroban-testnet.stellar.org',
 *   signer: freighterSigner
 * });
 * </script>
 * ```
 */
export function provideStellarGrants(
  config: StellarGrantsSDKConfig,
  logger?: StellarGrantsContext["logger"]
): void {
  const sdk = new StellarGrantsSDK(config);
  const context: StellarGrantsContext = { sdk, logger };
  provide(STELLAR_GRANTS_KEY, context);
}

/**
 * Internal helper to get the injected context.
 * Throws an error if used outside of a provider.
 */
export function getStellarGrantsContext(): StellarGrantsContext {
  const context = inject(STELLAR_GRANTS_KEY);
  if (!context) {
    throw new Error(
      "useStellarGrants must be used within a component tree that has called provideStellarGrants"
    );
  }
  return context;
}
