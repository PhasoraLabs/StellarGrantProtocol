import type { Ref } from "vue";
import { ref, onMounted } from "vue";
import { useStellarGrants } from "./useStellarGrants";
import type { GrantHistoryRecord, HistoryOptions } from "../types";

export interface UseTransactionHistoryOptions extends HistoryOptions {
  /** Fetch automatically on mount. Default: true */
  enabled?: boolean;
}

export interface UseTransactionHistoryResult {
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
 * Vue composable for fetching transaction history for a wallet address (#483).
 *
 * Uses `sdk.getTransactionHistory` and provides paginated reactive state.
 *
 * @param address - Stellar wallet address (G…)
 * @param options - Pagination, ordering, and fetch control
 *
 * @example
 * ```vue
 * <script setup>
 * import { useTransactionHistory } from '@stellargrants/client-sdk';
 *
 * const { records, isLoading, error, fetchMore } =
 *   useTransactionHistory('GABC...', { limit: 20 });
 * </script>
 *
 * <template>
 *   <div v-for="r in records" :key="r.txHash">
 *     {{ r.createdAt }} — {{ r.operationType }} ({{ r.successful ? 'ok' : 'fail' }})
 *   </div>
 *   <button @click="fetchMore">Load more</button>
 * </template>
 * ```
 */
export function useTransactionHistory(
  address: string,
  options: UseTransactionHistoryOptions = {},
): UseTransactionHistoryResult {
  const { enabled = true, limit = 50, order = "desc" } = options;
  const { sdk, logger } = useStellarGrants();

  const records = ref<GrantHistoryRecord[]>([]);
  const isLoading = ref(false);
  const error = ref<Error | null>(null);
  const nextCursor = ref<string | undefined>(undefined);

  const fetch = async (cursor?: string, append = false): Promise<void> => {
    isLoading.value = true;
    error.value = null;
    try {
      logger?.debug("Fetching transaction history", { address, cursor });
      const result = await sdk.getTransactionHistory(address, { limit, order, cursor });
      records.value = append ? [...records.value, ...result.records] : result.records;
      nextCursor.value = result.nextCursor;
      logger?.info("Transaction history fetched", {
        address,
        count: result.records.length,
      });
    } catch (err) {
      const errObj = err instanceof Error ? err : new Error(String(err));
      error.value = errObj;
      logger?.error("Error fetching transaction history", {
        address,
        error: errObj.message,
      });
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

  return { records, isLoading, error, nextCursor, fetchMore, refetch };
}
