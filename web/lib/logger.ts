/**
 * SDK Logger
 *
 * Configurable structured logger for StellarGrants SDK interactions.
 * Silent in production by default; enable debug mode via StellarGrantsProvider
 * or by setting NEXT_PUBLIC_SDK_DEBUG=true.
 */

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogEntry {
  level: LogLevel;
  message: string;
  context?: Record<string, unknown>;
  timestamp: string;
}

export interface ILogger {
  debug(message: string, context?: Record<string, unknown>): void;
  info(message: string, context?: Record<string, unknown>): void;
  warn(message: string, context?: Record<string, unknown>): void;
  error(message: string, context?: Record<string, unknown>): void;
}

const LEVELS: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

const SENSITIVE_KEYS = new Set(["secret", "privateKey", "seed", "mnemonic", "password"]);

function maskSensitive(context?: Record<string, unknown>): Record<string, unknown> | undefined {
  if (!context) return undefined;
  const masked: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(context)) {
    masked[k] = SENSITIVE_KEYS.has(k) ? "***" : v;
  }
  return masked;
}

export class Logger implements ILogger {
  private minLevel: LogLevel;
  private prefix: string;

  constructor(config?: { level?: LogLevel; prefix?: string }) {
    this.minLevel = config?.level ?? (this.isDebugEnabled() ? "debug" : "warn");
    this.prefix = config?.prefix ?? "[StellarGrants]";
  }

  private isDebugEnabled(): boolean {
    if (typeof process !== "undefined" && process.env.NEXT_PUBLIC_SDK_DEBUG === "true") return true;
    if (typeof window !== "undefined" && (window as unknown as Record<string, unknown>).__STELLAR_GRANTS_DEBUG__) return true;
    return false;
  }

  private shouldLog(level: LogLevel): boolean {
    return LEVELS[level] >= LEVELS[this.minLevel];
  }

  private format(level: LogLevel, message: string, context?: Record<string, unknown>): LogEntry {
    return {
      level,
      message,
      context: maskSensitive(context),
      timestamp: new Date().toISOString(),
    };
  }

  debug(message: string, context?: Record<string, unknown>): void {
    if (!this.shouldLog("debug")) return;
    const entry = this.format("debug", message, context);
    console.debug(`${this.prefix} [DEBUG] ${entry.message}`, entry.context ?? "");
  }

  info(message: string, context?: Record<string, unknown>): void {
    if (!this.shouldLog("info")) return;
    const entry = this.format("info", message, context);
    console.info(`${this.prefix} [INFO] ${entry.message}`, entry.context ?? "");
  }

  warn(message: string, context?: Record<string, unknown>): void {
    if (!this.shouldLog("warn")) return;
    const entry = this.format("warn", message, context);
    console.warn(`${this.prefix} [WARN] ${entry.message}`, entry.context ?? "");
  }

  error(message: string, context?: Record<string, unknown>): void {
    if (!this.shouldLog("error")) return;
    const entry = this.format("error", message, context);
    console.error(`${this.prefix} [ERROR] ${entry.message}`, entry.context ?? "");
  }

  /** Return a child logger with an additional namespace prefix */
  child(namespace: string): Logger {
    return new Logger({ level: this.minLevel, prefix: `${this.prefix}:${namespace}` });
  }

  /** Change the minimum log level at runtime */
  setLevel(level: LogLevel): void {
    this.minLevel = level;
  }
}

/** Shared default logger instance */
export const logger = new Logger();
