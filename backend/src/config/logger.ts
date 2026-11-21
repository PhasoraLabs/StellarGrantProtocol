import winston from "winston";
import { env } from "./env";

const logLevel = env.logLevel;
const isProduction = env.nodeEnv === "production";
const isTest = env.nodeEnv === "test";

/**
 * Determines whether structured JSON logging format is enabled.
 * Can be explicitly toggled via LOG_FORMAT=json, LOG_JSON=true, or
 * STRUCTURED_LOGGING=true, and defaults to JSON in production environments
 * unless explicitly disabled (e.g. LOG_FORMAT=text or LOG_JSON=false).
 */
export function isJsonLoggingEnabled(
  envSource: NodeJS.ProcessEnv = process.env,
): boolean {
  const format = (envSource.LOG_FORMAT ?? "").toLowerCase().trim();
  const logJson = (envSource.LOG_JSON ?? "").toLowerCase().trim();
  const structured = (envSource.STRUCTURED_LOGGING ?? "").toLowerCase().trim();
  const nodeEnv = envSource.NODE_ENV ?? env.nodeEnv;

  // Explicit positive toggles
  if (
    format === "json" ||
    logJson === "true" ||
    logJson === "1" ||
    structured === "true" ||
    structured === "1"
  ) {
    return true;
  }

  // Explicit negative toggles
  if (
    format === "text" ||
    format === "pretty" ||
    logJson === "false" ||
    logJson === "0" ||
    structured === "false" ||
    structured === "0"
  ) {
    return false;
  }

  // Default to JSON in production, text otherwise
  return nodeEnv === "production";
}

/**
 * Builds the Winston log format for general application logging.
 */
export function buildLogFormat(options: { isJson?: boolean } = {}) {
  const useJson = options.isJson ?? isJsonLoggingEnabled();

  return winston.format.combine(
    winston.format.timestamp({ format: "isoDateTime" }),
    winston.format.errors({ stack: true }),
    useJson
      ? winston.format.json()
      : winston.format.combine(
          winston.format.colorize(),
          winston.format.printf(({ level, message, timestamp, stack, ...metadata }) => {
            let msg = `${timestamp} [${level}]: ${message}`;
            if (stack) {
              msg += `\n${stack}`;
            }
            if (Object.keys(metadata).length > 0) {
              msg += `\n${JSON.stringify(metadata, null, 2)}`;
            }
            return msg;
          }),
        ),
  );
}

/**
 * Builds the Winston log format for HTTP request logging.
 */
export function buildRequestLogFormat(options: { isJson?: boolean } = {}) {
  const useJson = options.isJson ?? isJsonLoggingEnabled();

  return useJson
    ? winston.format.combine(
        winston.format.timestamp({ format: "isoDateTime" }),
        winston.format.json(),
      )
    : winston.format.combine(
        winston.format.colorize(),
        winston.format.printf(({ level, message, timestamp, ...metadata }) => {
          return `${timestamp} [${level}]: ${message} ${JSON.stringify(metadata)}`;
        }),
      );
}

/**
 * Factory helper for creating logger instances with custom format/level settings.
 */
export function createConfiguredLogger(options: {
  isJson?: boolean;
  level?: string;
  silent?: boolean;
} = {}): winston.Logger {
  return winston.createLogger({
    level: options.level ?? logLevel,
    format: buildLogFormat(options),
    transports: [
      new winston.transports.Console({
        silent: options.silent ?? isTest,
      }),
    ],
  });
}

export const logger = createConfiguredLogger();

export const requestLogger = (): winston.Logger =>
  winston.createLogger({
    level: logLevel,
    format: buildRequestLogFormat(),
    transports: [
      new winston.transports.Console({
        silent: isTest,
      }),
    ],
  });
