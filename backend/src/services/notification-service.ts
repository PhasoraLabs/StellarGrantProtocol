import { Server as SocketServer, type Socket } from "socket.io";
import { Server as HttpServer } from "http";
import { env } from "../config/env";
import jwt from "jsonwebtoken";

export class NotificationService {
  private io: SocketServer | null = null;
  private userSockets: Map<string, string[]> = new Map();

  initialize(server: HttpServer): void {
    // Issue #1195: Require JWT_SECRET to be set at startup
    if (!env.jwtSecret || env.jwtSecret.trim() === "") {
      throw new Error("JWT_SECRET must be set to a non-empty value for WebSocket authentication");
    }

    this.io = new SocketServer(server, {
      cors: {
        origin: env.corsOrigins,
        methods: ["GET", "POST"],
        credentials: true,
      },
    });

    this.io.on("connection", (socket: Socket) => {
      // Authenticate by JWT token only - no query param fallback
      let address: string | undefined;

      try {
        const token = (socket.handshake.auth && (socket.handshake.auth as any).token) ||
          (socket.handshake.headers && socket.handshake.headers.authorization && String(socket.handshake.headers.authorization).replace(/^Bearer\s+/i, ""));

        if (!token) {
          console.warn(`WebSocket connection rejected: no token provided for socket ${socket.id}`);
          socket.disconnect(true);
          return;
        }

        try {
          const decoded = jwt.verify(token, env.jwtSecret);
          if (decoded && (decoded as any).stellarAddress) {
            address = (decoded as any).stellarAddress;
          } else {
            console.warn(`WebSocket connection rejected: invalid token payload for socket ${socket.id}`);
            socket.disconnect(true);
            return;
          }
        } catch (e) {
          console.warn(`WebSocket connection rejected: token verification failed for socket ${socket.id}`, e);
          socket.disconnect(true);
          return;
        }
      } catch (e) {
        console.warn(`WebSocket auth parse error for socket ${socket.id}`, e);
        socket.disconnect(true);
        return;
      }

      // Only proceed if we have a verified address
      if (address) {
        const sockets = this.userSockets.get(address) || [];
        sockets.push(socket.id);
        this.userSockets.set(address, sockets);
        console.log(`User ${address} connected with socket ${socket.id}`);

        socket.on("disconnect", () => {
          const updatedSockets = (this.userSockets.get(address!) || []).filter(
            (id) => id !== socket.id
          );
          if (updatedSockets.length === 0) {
            this.userSockets.delete(address!);
          } else {
            this.userSockets.set(address!, updatedSockets);
          }
          console.log(`User ${address} disconnected socket ${socket.id}`);
        });
      } else {
        console.warn(`WebSocket connection rejected: no address in verified token for socket ${socket.id}`);
        socket.disconnect(true);
      }
    });
  }

  notifyUser(address: string, type: string, payload: any): void {
    if (!this.io) return;

    const sockets = this.userSockets.get(address);
    if (sockets && sockets.length > 0) {
      sockets.forEach((socketId) => {
        this.io?.to(socketId).emit("notification", {
          type,
          payload,
          timestamp: new Date().toISOString(),
        });
      });
      console.log(`Notification sent to ${address}: ${type}`);
    } else {
      console.log(`No active sockets for user ${address}, notification cached/skipped`);
    }
  }

  broadcast(type: string, payload: any): void {
    if (!this.io) return;
    this.io.emit("notification", {
      type,
      payload,
      timestamp: new Date().toISOString(),
    });
    console.log(`Broadcasted notification: ${type}`);
  }
}

export const notificationService = new NotificationService();
