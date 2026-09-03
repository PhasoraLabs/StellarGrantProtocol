import type { InjectionKey } from "vue";
import { StellarGrantsSDK } from "../StellarGrantsSDK";

/**
 * Context value provided by StellarGrantsProvider.
 * Mirrors the React context structure for Vue compatibility.
 */
export interface StellarGrantsContext {
  /** Pre-configured StellarGrantsSDK instance */
  sdk: StellarGrantsSDK;
  /** Optional logger instance for debugging */
  logger?: {
    debug: (...args: unknown[]) => void;
    info: (...args: unknown[]) => void;
    warn: (...args: unknown[]) => void;
    error: (...args: unknown[]) => void;
  };
}

/**
 * Injection key symbol for StellarGrants context.
 * Used internally by provide/inject mechanism.
 */
export const STELLAR_GRANTS_KEY: InjectionKey<StellarGrantsContext> =
  Symbol("stellar-grants");
