import type { Ref } from "vue";
import { ref, onUnmounted } from "vue";
import { useStellarGrants } from "./useStellarGrants";
import type { GrantBalances } from "../types";

export interface UseGrantBalancesOptions {
  /** Poll interval in milliseconds. Default: 10_000 (10 seconds) */
  pollInterval?: number;
  /** Fetch immediately on composable setup. Default: true */
  immediate?: boolean;
}

export interface UseGrantBalancesResult {
  data: Ref<GrantBalances | null>;
  isLoading: Ref<boolean>;
  error: Ref<Error | null>;
  /** Manually trigger a one-shot balance refresh */
  refetch: () => Promise<void>;
  /** Stop the polling listener */
  stop: () => void;
}

/**
 * Vue composable for real-time grant balance monitoring (#489).
 *
 * Starts a polling listener via `sdk.listenToGrantBalanceChanges` and exposes
 * reactive refs for the latest snapshot, loading state, and any error.
 *
 * @param grantId - The grant whose contract balances to monitor
 * @param options - Polling and initialization options
 *
 * @example
 * ```vue
 * <script setup>
 * import { useGrantBalances } from '@stellargrants/client-sdk';
 *
 * const { data: balances, isLoading, error } = useGrantBalances(1);
 * </script>
 *
 * <template>
 *   <div v-if="isLoading">Loading balances…</div>
 *   <div v-else-if="error">{{ error.message }}</div>
 *   <ul v-else>
 *     <li v-for="b in balances?.balances" :key="b.assetCode">
 *       {{ b.assetCode }}: {{ b.formatted }}
 *     </li>
 *   </ul>
 * </template>
 * ```
 */
export function useGrantBalances(
  grantId: number,
  options: UseGrantBalancesOptions = {},
): UseGrantBalancesResult {
  const { pollInterval = 10_000, immediate = true } = options;
  const { sdk, logger } = useStellarGrants();

  const data = ref<GrantBalances | null>(null);
  const isLoading = ref(false);
  const error = ref<Error | null>(null);

  const refetch = async (): Promise<void> => {
    isLoading.value = true;
    error.value = null;
    try {
      logger?.debug("Fetching grant balances", { grantId });
      data.value = await sdk.getGrantBalances(grantId);
      logger?.info("Grant balances fetched", {
        grantId,
        count: data.value.balances.length,
      });
    } catch (err) {
      const errObj = err instanceof Error ? err : new Error(String(err));
      error.value = errObj;
      logger?.error("Error fetching grant balances", { grantId, error: errObj.message });
    } finally {
      isLoading.value = false;
    }
  };

  let stopListener: (() => void) | null = null;

  const start = () => {
    if (immediate) {
      void refetch();
    }
    stopListener = sdk.listenToGrantBalanceChanges(grantId, {
      pollInterval,
      onChange: (current) => {
        data.value = current;
        error.value = null;
        logger?.debug("Grant balance changed", { grantId });
      },
      onError: (err) => {
        error.value = err;
        logger?.error("Balance listener error", { grantId, error: err.message });
      },
    });
  };

  const stop = () => {
    stopListener?.();
    stopListener = null;
  };

  start();
  onUnmounted(stop);

  return { data, isLoading, error, refetch, stop };
}
