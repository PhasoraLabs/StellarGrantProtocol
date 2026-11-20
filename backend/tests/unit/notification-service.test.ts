import { describe, expect, it, beforeAll } from "vitest";
import { Server as HttpServer } from "http";
import { Server as SocketServer } from "socket.io";
import { io as ioClient, Socket as ClientSocket } from "socket.io-client";
import { NotificationService } from "../../src/services/notification-service";
import { env } from "../../src/config/env";

describe("NotificationService WebSocket authentication (#1195)", () => {
  let httpServer: HttpServer;
  let notificationService: NotificationService;
  let serverUrl: string;

  beforeAll(() => {
    // Set a valid JWT_SECRET for testing
    env.jwtSecret = "test-secret-key-for-testing-purposes-only";
  });

  it("rejects connections without a valid JWT token", async () => {
    // Create a simple HTTP server
    httpServer = new HttpServer();
    notificationService = new NotificationService();

    // Initialize the notification service
    notificationService.initialize(httpServer);

    // Start the server
    httpServer.listen(0);
    const address = httpServer.address() as any;
    serverUrl = `http://localhost:${address.port}`;

    // Try to connect without a token
    const client = ioClient(serverUrl);

    // Wait for connection or error
    await new Promise<void>((resolve) => {
      client.on("connect_error", (err) => {
        expect(err.message).toMatch(/transport error/i);
        resolve();
      });
      client.on("connect", () => {
        // Should not reach here
        client.disconnect();
        resolve();
      });
    });

    client.disconnect();
    httpServer.close();
  });

  it("rejects connections with only query.address parameter (no JWT)", async () => {
    httpServer = new HttpServer();
    notificationService = new NotificationService();
    notificationService.initialize(httpServer);
    httpServer.listen(0);
    const address = httpServer.address() as any;
    serverUrl = `http://localhost:${address.port}`;

    // Try to connect with only query param (old vulnerable behavior)
    const client = ioClient(serverUrl, {
      query: { address: "GBRPYHIL2C2WBO36G6UIGR2PA4M3TQ7VOY3RTMAL4LRRA67ZOHQ65SZD" },
    });

    await new Promise<void>((resolve) => {
      client.on("connect_error", (err) => {
        expect(err.message).toMatch(/transport error/i);
        resolve();
      });
      client.on("connect", () => {
        client.disconnect();
        resolve();
      });
    });

    client.disconnect();
    httpServer.close();
  });

  it("rejects connection when JWT_SECRET is not set", () => {
    // Temporarily clear JWT_SECRET
    const originalSecret = env.jwtSecret;
    env.jwtSecret = "";

    httpServer = new HttpServer();
    notificationService = new NotificationService();

    expect(() => {
      notificationService.initialize(httpServer);
    }).toThrow("JWT_SECRET must be set");

    // Restore
    env.jwtSecret = originalSecret;
  });
});
