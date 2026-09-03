/**
 * Shared Utility Functions
 *
 * Common helper functions used across the application.
 */

/**
 * Format Stellar address for display
 */
export function formatAddress(address: string, truncate: number = 8): string {
  if (address.length <= truncate * 2) return address;
  return `${address.slice(0, truncate)}...${address.slice(-truncate)}`;
}

/**
 * Format amount from stroops to display format
 */
export function formatAmount(stroops: bigint, decimals: number = 7): string {
  const divisor = BigInt(10 ** decimals);
  const whole = stroops / divisor;
  const fractional = stroops % divisor;
  return `${whole}.${fractional.toString().padStart(decimals, "0")}`;
}

/**
 * Format date for display
 */
export function formatDate(date: Date | number | string): string {
  const d = typeof date === "string" || typeof date === "number" ? new Date(date) : date;
  return d.toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

/**
 * Calculate percentage
 */
export function calculatePercentage(current: number, total: number): number {
  if (total === 0) return 0;
  return Math.min((current / total) * 100, 100);
}

// Optimistic UI state management
export {
  TransactionTracker,
  OptimisticStore,
  predictGrantState,
  createOptimisticGrantMutation,
} from "./optimistic";
export type {
  TransactionStage,
  TransactionEvent,
  TransactionListener,
  GrantMutation,
} from "./optimistic";
