import { getStellarGrantsContext } from "./provideStellarGrants";
import type { StellarGrantsContext } from "./types";

/**
 * Vue composable to access the StellarGrants SDK context.
 * Must be used within a component tree that has called provideStellarGrants.
 *
 * @returns The StellarGrantsContext containing the SDK instance and optional logger
 *
 * @example
 * ```vue
 * <script setup>
 * import { useStellarGrants } from '@stellargrants/client-sdk';
 * 
 * const { sdk, logger } = useStellarGrants();
 * 
 * const createGrant = async () => {
 *   await sdk.grantCreate({
 *     owner: 'G...',
 *     title: 'My Grant',
 *     description: 'Description',
 *     budget: 1000000n,
 *     deadline: 1234567890n,
 *     milestoneCount: 3
 *   });
 * };
 * </script>
 * ```
 */
export function useStellarGrants(): StellarGrantsContext {
  return getStellarGrantsContext();
}
