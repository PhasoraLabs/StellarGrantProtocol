import type { Ref } from "vue";
import { ref, onMounted, onUnmounted } from "vue";
import { useStellarGrants } from "./useStellarGrants";
import type { GrantData } from "../types";

/**
 * Options for the useGrants composable.
 */
export interface UseGrantsOptions {
  /** Whether to automatically fetch grants on mount. Default: true */
  enabled?: boolean;
  /** Refetch interval in milliseconds. Default: 30000 (30 seconds) */
  refetchInterval?: number;
}

/**
 * Result returned by the useGrants composable.
 */
export interface UseGrantsResult {
  /** Array of grant data */
  data: Ref<GrantData[]>;
  /** Whether a fetch operation is in progress */
  isLoading: Ref<boolean>;
  /** Error from the last fetch operation, if any */
  error: Ref<Error | null>;
  /** Manually trigger a refetch */
  refetch: () => Promise<void>;
}

/**
 * Vue composable for fetching and managing grants.
 * Provides reactive state for grants list with automatic refetching.
 *
 * @param options - Configuration options
 * @returns Reactive state and methods for grants management
 *
 * @example
 * ```vue
 * <script setup>
 * import { useGrants } from '@stellargrants/client-sdk';
 * 
 * const { data: grants, isLoading, error, refetch } = useGrants({
 *   enabled: true,
 *   refetchInterval: 30000
 * });
 * </script>
 * 
 * <template>
 *   <div v-if="isLoading">Loading grants...</div>
 *   <div v-else-if="error">Error: {{ error.message }}</div>
 *   <div v-else>
 *     <div v-for="grant in grants" :key="grant.id">
 *       {{ grant.title }}
 *     </div>
 *   </div>
 * </template>
 * ```
 */
export function useGrants(options: UseGrantsOptions = {}): UseGrantsResult {
  const { enabled = true, refetchInterval = 30000 } = options;
  const { sdk, logger } = useStellarGrants();

  const data = ref<GrantData[]>([]);
  const isLoading = ref(false);
  const error = ref<Error | null>(null);

  const fetchGrants = async (): Promise<void> => {
    if (!enabled) return;

    isLoading.value = true;
    error.value = null;

    try {
      // Note: This is a placeholder implementation.
      // The actual SDK needs a method to fetch all grants.
      // For now, we'll simulate with individual grant fetches.
      // In a real implementation, you'd call a method like sdk.getAllGrants()
      // or fetch from an API endpoint.
      
      logger?.debug("Fetching grants");
      
      // Placeholder: empty array until SDK has batch fetch method
      data.value = [];
      
      logger?.info("Grants fetched", { count: data.value.length });
    } catch (err) {
      const errObj = err instanceof Error ? err : new Error(String(err));
      error.value = errObj;
      logger?.error("Error fetching grants", { error: errObj.message });
    } finally {
      isLoading.value = false;
    }
  };

  onMounted(() => {
    if (enabled) {
      fetchGrants();
    }
  });

  // Set up refetch interval
  let intervalId: ReturnType<typeof setInterval> | null = null;
  if (enabled && refetchInterval > 0) {
    intervalId = setInterval(fetchGrants, refetchInterval);
  }

  // Cleanup on unmount
  onUnmounted(() => {
    if (intervalId) {
      clearInterval(intervalId);
    }
  });

  return {
    data,
    isLoading,
    error,
    refetch: fetchGrants,
  };
}
