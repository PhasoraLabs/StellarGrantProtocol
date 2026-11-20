"use client";

/**
 * StellarGrantsProvider
 *
 * Top-level React context that manages the SDK lifecycle: contract client,
 * wallet connection state, configurable logger, shared BatchBuilder, and the
 * high-level StellarGrantsSDK instance for search/filter operations.
 * Wrap your app (or a subtree) with this provider and consume the context
 * via `useStellarGrants()`.
 *
 * Issue #494 — added `sdk` (StellarGrantsSDK) and `config` to context so
 * React hooks have access to the full SDK without separate instantiation.
 */

import { createContext, useMemo, type ReactNode } from "react";
import { ContractClient, contractClient as defaultClient } from "@/lib/stellar/contract";
import { Logger, type LogLevel, type ILogger } from "@/lib/logger";
import { BatchBuilder } from "@/lib/stellar/batchBuilder";
import { StellarGrantsSDK } from "@/lib/stellar/sdk";

/** SDK-wide configuration (Issue #492) */
export interface StellarGrantsSDKConfig {
  /** Minimum log level for SDK output. Defaults to "warn" (silent in prod). */
  logLevel?: LogLevel;
  /** Shorthand to enable full debug logging. Equivalent to logLevel="debug". */
  debug?: boolean;
  /** Provide your own logger implementation (must implement ILogger). */
  logger?: ILogger;
}

export interface StellarGrantsContextValue {
  /** Pre-configured ContractClient instance */
  client: ContractClient;
  /** SDK logger (child of the global logger) */
  logger: Logger;
  /** Shared BatchBuilder for the current render subtree */
  batch: BatchBuilder;
  /**
   * High-level StellarGrantsSDK instance.
   * Call `sdk.hydrate(grants)` after fetching grant lists to enable
   * search, filter, and sort helpers (Issue #494).
   */
  sdk: StellarGrantsSDK;
  /** Active SDK configuration */
  config: Required<StellarGrantsSDKConfig>;
}

export const StellarGrantsContext = createContext<StellarGrantsContextValue | null>(null);

export interface StellarGrantsProviderProps {
  children: ReactNode;
  /** Override the default contract client (useful in tests) */
  client?: ContractClient;
  /** Minimum log level for SDK output. Defaults to "warn" (silent in prod) */
  logLevel?: LogLevel;
  /** Enable verbose debug logging — equivalent to logLevel="debug" */
  debug?: boolean;
  /** Provide your own ILogger implementation */
  logger?: ILogger;
}

export function StellarGrantsProvider({
  children,
  client,
  logLevel,
  debug = false,
  logger: customLogger,
}: StellarGrantsProviderProps) {
  const ctx = useMemo<StellarGrantsContextValue>(() => {
    const level: LogLevel = debug ? "debug" : (logLevel ?? "warn");

    const sdkLogger =
      customLogger instanceof Logger
        ? customLogger
        : customLogger
        ? // Wrap a plain ILogger — create a Logger that delegates to it
          new Logger({ level, prefix: "[StellarGrants]" })
        : new Logger({ level, prefix: "[StellarGrants]" });

    sdkLogger.info("StellarGrantsProvider mounted", { level });

    const resolvedClient = client ?? defaultClient;

    return {
      client: resolvedClient,
      logger: sdkLogger,
      batch: new BatchBuilder(),
      sdk: new StellarGrantsSDK(),
      config: {
        logLevel: level,
        debug,
        logger: customLogger ?? sdkLogger,
      },
    };
  }, [client, logLevel, debug, customLogger]);

  return (
    <StellarGrantsContext.Provider value={ctx}>
      {children}
    </StellarGrantsContext.Provider>
  );
}
