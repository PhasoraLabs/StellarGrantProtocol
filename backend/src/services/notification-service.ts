import { Server as SocketServer, type Socket } from "socket.io";
import { Server as HttpServer } from "http";
import { env } from "../config/env";
import jwt from "jsonwebtoken";

export class NotificationService {
  private io: SocketServer | null = null;
  private userSockets: Map<string, string[]> = new Map();

  initialize(server: HttpServer): void {
    this.io = new SocketServer(server, {
      cors: {
        origin: env.corsOrigins,
        methods: ["GET", "POST"],
        credentials: true,
      },
    });

    this.io.on("connection", (socket: Socket) => {
      // Authenticate by JWT token (preferred) or by `address` query param.
      let address: string | undefined;

      try {
        const token = (socket.handshake.auth && (socket.handshake.auth as any).token) ||
          (socket.handshake.headers && socket.handshake.headers.authorization && String(socket.handshake.headers.authorization).replace(/^Bearer\s+/i, ""));
        if (token) {
          if (!env.jwtSecret) {
            console.warn("JWT_SECRET not set; accepting websocket connection without verification");
          }
          try {
            const decoded = token && env.jwtSecret ? jwt.verify(token, env.jwtSecret) : null;
            if (decoded && (decoded as any).stellarAddress) {
              address = (decoded as any).stellarAddress;
            }
          } catch (e) {
            console.warn("WebSocket token verification failed", e);
          }
        }
      } catch (e) {
        console.warn("WebSocket auth parse error", e);
      }

      // fallback to query.address when token didn't provide it
      if (!address) {
        address = socket.handshake.query.address as string | undefined;
      }

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
        console.log(`Anonymous socket ${socket.id} connected`);
        socket.on("disconnect", () => console.log(`Anonymous socket ${socket.id} disconnected`));
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
