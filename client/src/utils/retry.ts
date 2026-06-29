import type { RetryConfig } from "../types";

/**
 * Default retry configuration values.
 */
const DEFAULT_RETRY_CONFIG: Required<RetryConfig> = {
  maxAttempts: 3,
  initialDelayMs: 1000,
  backoffMultiplier: 2,
  maxDelayMs: 10000,
  retryOnRateLimit: true,
  retryOnTimeout: true,
  retryOnNetworkError: true,
  onRetry: (attempt: number, error: Error, delayMs: number) => {
    console.debug(
      `[StellarGrants] RPC retry attempt ${attempt} after ${delayMs}ms — ${error.message}`,
    );
  },
};

/**
 * Checks if an error should trigger a retry based on the configuration.
 */
function shouldRetry(error: unknown, config: Required<RetryConfig>): boolean {
  const errorMessage = error instanceof Error ? error.message : String(error);
  const errorString = errorMessage.toLowerCase();

  // Check for rate limit errors (HTTP 429 or rate limit messages)
  if (config.retryOnRateLimit) {
    if (errorString.includes("429") || errorString.includes("rate limit") || errorString.includes("too many requests")) {
      return true;
    }
  }

  // Check for timeout errors
  if (config.retryOnTimeout) {
    if (errorString.includes("timeout") || errorString.includes("timed out") || errorString.includes("etimedout")) {
      return true;
    }
  }

  // Check for generic network errors
  if (config.retryOnNetworkError) {
    if (
      errorString.includes("network") ||
      errorString.includes("econnrefused") ||
      errorString.includes("enotfound") ||
      errorString.includes("econnreset") ||
      errorString.includes("fetch failed") ||
      errorString.includes("server busy")
    ) {
      return true;
    }
  }

  return false;
}

/**
 * Calculates the delay for the next retry attempt using exponential backoff.
 */
function calculateDelay(attempt: number, config: Required<RetryConfig>): number {
  const delay = config.initialDelayMs * Math.pow(config.backoffMultiplier, attempt - 1);
  return Math.min(delay, config.maxDelayMs);
}

/**
 * Executes a function with automatic retry logic and exponential backoff.
 *
 * @param fn - The async function to execute
 * @param config - Retry configuration
 * @returns The result of the function
 * @throws The last error if all retry attempts are exhausted
 *
 * @example
 * ```typescript
 * const result = await retryWithBackoff(
 *   () => fetch('https://api.example.com/data'),
 *   { maxAttempts: 5, initialDelayMs: 500 }
 * );
 * ```
 */
export async function retryWithBackoff<T>(
  fn: () => Promise<T>,
  config?: RetryConfig
): Promise<T> {
  const effectiveConfig = { ...DEFAULT_RETRY_CONFIG, ...config };

  let lastError: Error | null = null;

  for (let attempt = 1; attempt <= effectiveConfig.maxAttempts; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));

      // If this was the last attempt, throw the error
      if (attempt === effectiveConfig.maxAttempts) {
        throw lastError;
      }

      // Check if we should retry this error
      if (!shouldRetry(error, effectiveConfig)) {
        throw lastError;
      }

      // Calculate delay and wait
      const delayMs = calculateDelay(attempt, effectiveConfig);
      
      // Log the retry attempt
      effectiveConfig.onRetry(attempt, lastError, delayMs);

      // Wait before retrying
      await new Promise((resolve) => setTimeout(resolve, delayMs));
    }
  }

  // This should never be reached, but TypeScript needs it
  throw lastError || new Error("Retry failed");
}

/**
 * Wraps a function to add retry logic.
 * Returns a new function that will retry on failure.
 *
 * @param fn - The async function to wrap
 * @param config - Retry configuration
 * @returns A wrapped function with retry logic
 *
 * @example
 * ```typescript
 * const fetchWithRetry = withRetry(
 *   (url: string) => fetch(url),
 *   { maxAttempts: 3 }
 * );
 * const result = await fetchWithRetry('https://api.example.com/data');
 * ```
 */
export function withRetry<T extends (...args: any[]) => Promise<any>>(
  fn: T,
  config?: RetryConfig
): T {
  return (async (...args: Parameters<T>): Promise<ReturnType<T>> => {
    return retryWithBackoff(() => fn(...args), config);
  }) as T;
}
