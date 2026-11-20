import type { Ref } from "vue";
import { ref, onMounted, onUnmounted, watch } from "vue";
import { useStellarGrants } from "./useStellarGrants";
import type { GrantData } from "../types";

/**
 * Options for the useGrant composable.
 */
export interface UseGrantOptions {
  /** Whether to automatically fetch the grant on mount. Default: true */
  enabled?: boolean;
  /** Refetch interval in milliseconds. Default: 30000 (30 seconds) */
  refetchInterval?: number;
}

/**
 * Result returned by the useGrant composable.
 */
export interface UseGrantResult {
  /** Grant data or null if not found */
  data: Ref<GrantData | null>;
  /** Whether a fetch operation is in progress */
  isLoading: Ref<boolean>;
  /** Error from the last fetch operation, if any */
  error: Ref<Error | null>;
  /** Manually trigger a refetch */
  refetch: () => Promise<void>;
}

/**
 * Vue composable for fetching and managing a single grant by ID.
 * Provides reactive state for a grant with automatic refetching.
 *
 * @param grantId - The ID of the grant to fetch
 * @param options - Configuration options
 * @returns Reactive state and methods for grant management
 *
 * @example
 * ```vue
 * <script setup>
 * import { useGrant } from '@stellargrants/client-sdk';
 * 
 * const { data: grant, isLoading, error, refetch } = useGrant(1, {
 *   enabled: true,
 *   refetchInterval: 30000
 * });
 * </script>
 * 
 * <template>
 *   <div v-if="isLoading">Loading grant...</div>
 *   <div v-else-if="error">Error: {{ error.message }}</div>
 *   <div v-else-if="grant">
 *     <h2>{{ grant.title }}</h2>
 *     <p>{{ grant.description }}</p>
 *   </div>
 * </template>
 * ```
 */
export function useGrant(grantId: number, options: UseGrantOptions = {}): UseGrantResult {
  const { enabled = true, refetchInterval = 30000 } = options;
  const { sdk, logger } = useStellarGrants();

  const data = ref<GrantData | null>(null);
  const isLoading = ref(false);
  const error = ref<Error | null>(null);

  const fetchGrant = async (): Promise<void> => {
    if (!enabled || !grantId) return;

    isLoading.value = true;
    error.value = null;

    try {
      logger?.debug("Fetching grant", { grantId });
      
      const grant = await sdk.grantGet(grantId);
      data.value = grant as any;
      
      logger?.info("Grant fetched", { grantId, status: (grant as any)?.status });
    } catch (err) {
      const errObj = err instanceof Error ? err : new Error(String(err));
      error.value = errObj;
      logger?.error("Error fetching grant", { grantId, error: errObj.message });
    } finally {
      isLoading.value = false;
    }
  };

  onMounted(() => {
    if (enabled) {
      fetchGrant();
    }
  });

  // Watch for grantId changes
  watch(() => grantId, () => {
    if (enabled) {
      fetchGrant();
    }
  });

  // Set up refetch interval
  let intervalId: ReturnType<typeof setInterval> | null = null;
  if (enabled && refetchInterval > 0) {
    intervalId = setInterval(fetchGrant, refetchInterval);
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
    refetch: fetchGrant,
  };
}
