import { describe, it, expect } from "vitest";
import winston from "winston";
import { Writable } from "stream";
import {
  isJsonLoggingEnabled,
  buildLogFormat,
  buildRequestLogFormat,
  createConfiguredLogger,
  logger,
  requestLogger,
} from "../../src/config/logger";

function captureLoggerOutput(createFn: (dest: Writable) => winston.Logger, logAction: (l: winston.Logger) => void): string {
  let output = "";
  const dest = new Writable({
    write(chunk, _enc, cb) {
      output += chunk.toString();
      cb();
    },
  });
  const custom = createFn(dest);
  logAction(custom);
  return output;
}

describe("config/logger - structured JSON logging option (#1423)", () => {
  describe("isJsonLoggingEnabled", () => {
    it("returns true when LOG_FORMAT=json", () => {
      expect(isJsonLoggingEnabled({ LOG_FORMAT: "json", NODE_ENV: "development" })).toBe(true);
      expect(isJsonLoggingEnabled({ LOG_FORMAT: "JSON", NODE_ENV: "development" })).toBe(true);
    });

    it("returns true when LOG_JSON=true or 1", () => {
      expect(isJsonLoggingEnabled({ LOG_JSON: "true", NODE_ENV: "development" })).toBe(true);
      expect(isJsonLoggingEnabled({ LOG_JSON: "1", NODE_ENV: "development" })).toBe(true);
    });

    it("returns true when STRUCTURED_LOGGING=true or 1", () => {
      expect(isJsonLoggingEnabled({ STRUCTURED_LOGGING: "true", NODE_ENV: "development" })).toBe(true);
      expect(isJsonLoggingEnabled({ STRUCTURED_LOGGING: "1", NODE_ENV: "development" })).toBe(true);
    });

    it("defaults to true in production environment", () => {
      expect(isJsonLoggingEnabled({ NODE_ENV: "production" })).toBe(true);
    });

    it("returns false in production when explicitly disabled via LOG_FORMAT=text", () => {
      expect(isJsonLoggingEnabled({ NODE_ENV: "production", LOG_FORMAT: "text" })).toBe(false);
      expect(isJsonLoggingEnabled({ NODE_ENV: "production", LOG_FORMAT: "pretty" })).toBe(false);
    });

    it("returns false in production when explicitly disabled via LOG_JSON=false", () => {
      expect(isJsonLoggingEnabled({ NODE_ENV: "production", LOG_JSON: "false" })).toBe(false);
      expect(isJsonLoggingEnabled({ NODE_ENV: "production", LOG_JSON: "0" })).toBe(false);
    });

    it("returns false in production when explicitly disabled via STRUCTURED_LOGGING=false", () => {
      expect(isJsonLoggingEnabled({ NODE_ENV: "production", STRUCTURED_LOGGING: "false" })).toBe(false);
    });

    it("defaults to false in development and test", () => {
      expect(isJsonLoggingEnabled({ NODE_ENV: "development" })).toBe(false);
      expect(isJsonLoggingEnabled({ NODE_ENV: "test" })).toBe(false);
    });
  });

  describe("output formatting", () => {
    it("formats output as valid parseable JSON when isJson=true", () => {
      const output = captureLoggerOutput(
        (dest) =>
          winston.createLogger({
            level: "info",
            format: buildLogFormat({ isJson: true }),
            transports: [new winston.transports.Stream({ stream: dest })],
          }),
        (l) => {
          l.info("Payment processed successfully", {
            grantId: 42,
            amount: "5000 XLM",
          });
        },
      );

      const parsed = JSON.parse(output.trim());
      expect(parsed.level).toBe("info");
      expect(parsed.message).toBe("Payment processed successfully");
      expect(parsed.grantId).toBe(42);
      expect(parsed.amount).toBe("5000 XLM");
      expect(parsed.timestamp).toBeDefined();
    });

    it("includes error stack trace in structured JSON format", () => {
      const err = new Error("Database connection timed out");
      const output = captureLoggerOutput(
        (dest) =>
          winston.createLogger({
            level: "error",
            format: buildLogFormat({ isJson: true }),
            transports: [new winston.transports.Stream({ stream: dest })],
          }),
        (l) => {
          l.error(err.message, { stack: err.stack });
        },
      );

      const parsed = JSON.parse(output.trim());
      expect(parsed.level).toBe("error");
      expect(parsed.message).toBe("Database connection timed out");
      expect(parsed.stack).toContain("Error: Database connection timed out");
    });

    it("formats output as human-readable text when isJson=false", () => {
      const output = captureLoggerOutput(
        (dest) =>
          winston.createLogger({
            level: "warn",
            format: buildLogFormat({ isJson: false }),
            transports: [new winston.transports.Stream({ stream: dest })],
          }),
        (l) => {
          l.warn("Rate limit near threshold", { ip: "127.0.0.1" });
        },
      );

      expect(output).toContain("Rate limit near threshold");
      expect(output).toContain('"ip": "127.0.0.1"');
      expect(() => JSON.parse(output)).toThrow();
    });

    it("produces parseable JSON for request logs when isJson=true", () => {
      const output = captureLoggerOutput(
        (dest) =>
          winston.createLogger({
            level: "info",
            format: buildRequestLogFormat({ isJson: true }),
            transports: [new winston.transports.Stream({ stream: dest })],
          }),
        (l) => {
          l.info("GET /grants/1 200 12ms", { status: 200, path: "/grants/1" });
        },
      );

      const parsed = JSON.parse(output.trim());
      expect(parsed.level).toBe("info");
      expect(parsed.status).toBe(200);
      expect(parsed.path).toBe("/grants/1");
      expect(parsed.timestamp).toBeDefined();
    });
  });

  describe("createConfiguredLogger & instances", () => {
    it("creates a functional winston logger instance", () => {
      const customLogger = createConfiguredLogger({ isJson: true, silent: true });
      expect(customLogger).toBeInstanceOf(winston.Logger);
      expect(typeof customLogger.info).toBe("function");
      expect(typeof customLogger.error).toBe("function");
    });

    it("exports default logger and requestLogger instances", () => {
      expect(logger).toBeInstanceOf(winston.Logger);
      const reqLogger = requestLogger();
      expect(reqLogger).toBeInstanceOf(winston.Logger);
    });
  });
});
