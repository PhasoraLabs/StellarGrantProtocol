import type { Ref } from "vue";
import { ref, onMounted, watch } from "vue";
import { useStellarGrants } from "./useStellarGrants";
import type { GrantHistoryRecord, HistoryOptions } from "../types";

export interface UseGrantHistoryOptions extends HistoryOptions {
  /** Fetch automatically on mount. Default: true */
  enabled?: boolean;
}

export interface UseGrantHistoryResult {
  records: Ref<GrantHistoryRecord[]>;
  isLoading: Ref<boolean>;
  error: Ref<Error | null>;
  nextCursor: Ref<string | undefined>;
  /** Fetch the next page, appending to existing records */
  fetchMore: () => Promise<void>;
  /** Reset and refetch from the beginning */
  refetch: () => Promise<void>;
}

/**
 * Vue composable for fetching the on-chain history for a specific grant (#483).
 *
 * Uses `sdk.getGrantHistory` which scopes the Horizon query to the contract
 * account and filters by memo convention (`grant:<id>`).
 *
 * @param grantId - Grant ID to retrieve history for
 * @param options - Pagination, ordering, and fetch control
 *
 * @example
 * ```vue
 * <script setup>
 * import { useGrantHistory } from '@stellargrants/client-sdk';
 *
 * const { records, isLoading, error } = useGrantHistory(42);
 * </script>
 *
 * <template>
 *   <div v-if="isLoading">Loading history…</div>
 *   <div v-for="r in records" :key="r.txHash">
 *     {{ r.operationType }} — {{ r.createdAt }}
 *   </div>
 * </template>
 * ```
 */
export function useGrantHistory(
  grantId: number,
  options: UseGrantHistoryOptions = {},
): UseGrantHistoryResult {
  const { enabled = true, limit = 50, order = "desc" } = options;
  const { sdk, logger } = useStellarGrants();

  const records = ref<GrantHistoryRecord[]>([]);
  const isLoading = ref(false);
  const error = ref<Error | null>(null);
  const nextCursor = ref<string | undefined>(undefined);

  const fetch = async (cursor?: string, append = false): Promise<void> => {
    if (!grantId) return;
    isLoading.value = true;
    error.value = null;
    try {
      logger?.debug("Fetching grant history", { grantId, cursor });
      const result = await sdk.getGrantHistory(grantId, { limit, order, cursor });
      records.value = append ? [...records.value, ...result.records] : result.records;
      nextCursor.value = result.nextCursor;
      logger?.info("Grant history fetched", { grantId, count: result.records.length });
    } catch (err) {
      const errObj = err instanceof Error ? err : new Error(String(err));
      error.value = errObj;
      logger?.error("Error fetching grant history", { grantId, error: errObj.message });
    } finally {
      isLoading.value = false;
    }
  };

  const refetch = () => {
    nextCursor.value = undefined;
    return fetch(undefined, false);
  };

  const fetchMore = () => fetch(nextCursor.value, true);

  onMounted(() => {
    if (enabled) void fetch();
  });

  watch(
    () => grantId,
    () => {
      if (enabled) {
        nextCursor.value = undefined;
        void fetch(undefined, false);
      }
    },
  );

  return { records, isLoading, error, nextCursor, fetchMore, refetch };
}
