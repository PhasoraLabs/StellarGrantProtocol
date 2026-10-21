import { describe, expect, it, beforeAll, vi } from "vitest";
import { Server as HttpServer } from "http";
import jwt from "jsonwebtoken";
import { NotificationService } from "../../src/services/notification-service";
import { env } from "../../src/config/env";

describe("NotificationService WebSocket authentication (#1195)", () => {
  let httpServer: HttpServer;
  let notificationService: NotificationService;

  beforeAll(() => {
    env.jwtSecret = "test-secret-key-for-testing-purposes-only";
  });

  const createMockSocket = (auth: any = {}, headers: any = {}) => ({
    id: "test-socket-1",
    handshake: { auth, headers },
    disconnect: vi.fn(),
    on: vi.fn(),
  });

  it("rejects connections without a valid JWT token", () => {
    httpServer = new HttpServer();
    notificationService = new NotificationService();
    notificationService.initialize(httpServer);

    const mockSocket = createMockSocket();
    (notificationService as any).io.emit("connection", mockSocket);

    expect(mockSocket.disconnect).toHaveBeenCalledWith(true);
  });

  it("rejects connections with only query.address parameter (no JWT)", () => {
    httpServer = new HttpServer();
    notificationService = new NotificationService();
    notificationService.initialize(httpServer);

    const mockSocket = createMockSocket();
    (mockSocket.handshake as any).query = { address: "GBRPYHIL2C2WBO36G6UIGR2PA4M3TQ7VOY3RTMAL4LRRA67ZOHQ65SZD" };
    (notificationService as any).io.emit("connection", mockSocket);

    expect(mockSocket.disconnect).toHaveBeenCalledWith(true);
  });

  it("accepts connections with a valid JWT token", () => {
    httpServer = new HttpServer();
    notificationService = new NotificationService();
    notificationService.initialize(httpServer);

    const token = jwt.sign(
      { stellarAddress: "GBRPYHIL2C2WBO36G6UIGR2PA4M3TQ7VOY3RTMAL4LRRA67ZOHQ65SZD" },
      env.jwtSecret
    );
    const mockSocket = createMockSocket({ token });
    (notificationService as any).io.emit("connection", mockSocket);

    expect(mockSocket.disconnect).not.toHaveBeenCalled();
  });

  it("rejects connection when JWT_SECRET is not set", () => {
    const originalSecret = env.jwtSecret;
    env.jwtSecret = "";

    httpServer = new HttpServer();
    notificationService = new NotificationService();

    expect(() => {
      notificationService.initialize(httpServer);
    }).toThrow("JWT_SECRET must be set");

    env.jwtSecret = originalSecret;
  });
});
