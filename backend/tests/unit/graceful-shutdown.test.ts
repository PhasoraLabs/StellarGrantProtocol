import { afterEach, describe, expect, it, vi } from "vitest";
import type { Server } from "node:http";
import type { DataSource } from "typeorm";
import { registerGracefulShutdown } from "../../src/utils/graceful-shutdown";

describe("registerGracefulShutdown", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it.each(["SIGINT", "SIGTERM"] as const)(
    "closes the HTTP server and database before exiting on %s",
    async (signal) => {
      const operations: string[] = [];
      let server: Server;
      server = {
        close: vi.fn((callback: (error?: Error) => void) => {
          operations.push("server.close");
          callback();
          return server;
        }),
      } as unknown as Server;
      const dataSource = {
        destroy: vi.fn(async () => {
          operations.push("dataSource.destroy");
        }),
      } as unknown as DataSource;
      const exit = vi.spyOn(process, "exit").mockImplementation((code) => {
        operations.push(`process.exit:${code ?? 0}`);
        return undefined as never;
      });

      const unregister = registerGracefulShutdown(server, dataSource);
      try {
        process.emit(signal);
        await vi.waitFor(() => expect(exit).toHaveBeenCalledWith(0));

        expect(operations).toEqual([
          "server.close",
          "dataSource.destroy",
          "process.exit:0",
        ]);
      } finally {
        unregister();
      }
    },
  );

  it("only starts shutdown once when multiple signals arrive", async () => {
    let server: Server;
    server = {
      close: vi.fn((callback: (error?: Error) => void) => {
        callback();
        return server;
      }),
    } as unknown as Server;
    const dataSource = {
      destroy: vi.fn().mockResolvedValue(undefined),
    } as unknown as DataSource;
    const exit = vi.spyOn(process, "exit").mockImplementation(
      () => undefined as never,
    );

    const unregister = registerGracefulShutdown(server, dataSource);
    try {
      process.emit("SIGTERM");
      process.emit("SIGINT");
      await vi.waitFor(() => expect(exit).toHaveBeenCalledOnce());

      expect(server.close).toHaveBeenCalledOnce();
      expect(dataSource.destroy).toHaveBeenCalledOnce();
    } finally {
      unregister();
    }
  });

  it("still closes the database and exits unsuccessfully if the server fails to close", async () => {
    const server = {
      close: vi.fn((callback: (error?: Error) => void) => {
        callback(new Error("close failed"));
        return server;
      }),
    } as unknown as Server;
    const dataSource = {
      destroy: vi.fn().mockResolvedValue(undefined),
    } as unknown as DataSource;
    const exit = vi
      .spyOn(process, "exit")
      .mockImplementation(() => undefined as never);
    const logError = vi.spyOn(console, "error").mockImplementation(() => {});

    const unregister = registerGracefulShutdown(server, dataSource);
    try {
      process.emit("SIGTERM");
      await vi.waitFor(() => expect(exit).toHaveBeenCalledWith(1));

      expect(dataSource.destroy).toHaveBeenCalledOnce();
      expect(logError).toHaveBeenCalledWith(
        "Failed to close HTTP server",
        expect.any(Error),
      );
    } finally {
      unregister();
    }
  });
});
