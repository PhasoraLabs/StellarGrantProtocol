/**
 * SSE Events Streaming Endpoint
 *
 * GET /events           — stream all grant-activity events
 * GET /events?grantId=1 — stream events filtered to a specific grant
 *
 * Browsers consume this via the native EventSource API without needing the
 * Socket.io client library. The endpoint uses long-polling against the
 * Activity table every 5 seconds and supports Last-Event-ID resumption.
 */

import { Router, Request, Response } from "express";
import { Repository, MoreThan } from "typeorm";
import { Activity, ActivityType } from "../entities/Activity";
import { metricsService } from "../services/metrics-service";

// Activity types that are surfaced over SSE (grant/milestone/dispute events only)
const STREAMED_TYPES = new Set<ActivityType>([
  "grant_created",
  "grant_funded",
  "grant_completed",
  "milestone_submitted",
  "milestone_approved",
  "milestone_reactivated",
]);

// Per-IP connection tracking for the 5-connections-per-IP cap
const connectionsByIp = new Map<string, number>();

const CONNECTION_LIMIT_PER_IP = 5;
const POLL_INTERVAL_MS = 5_000;
const KEEPALIVE_INTERVAL_MS = 25_000;

function getIp(req: Request): string {
  return req.ip ?? "unknown";
}

function incrementConnections(ip: string): boolean {
  const current = connectionsByIp.get(ip) ?? 0;
  if (current >= CONNECTION_LIMIT_PER_IP) return false;
  connectionsByIp.set(ip, current + 1);
  return true;
}

function decrementConnections(ip: string): void {
  const current = connectionsByIp.get(ip) ?? 0;
  if (current <= 1) {
    connectionsByIp.delete(ip);
  } else {
    connectionsByIp.set(ip, current - 1);
  }
}

export const buildEventsRouter = (activityRepo: Repository<Activity>) => {
  const router = Router();

  router.get("/", async (req: Request, res: Response) => {
    const ip = getIp(req);

    if (!incrementConnections(ip)) {
      res.status(429).json({
        error: "Too many SSE connections from this IP",
        limit: CONNECTION_LIMIT_PER_IP,
      });
      return;
    }

    metricsService.incrementSseConnections();

    // Parse optional grantId filter
    const grantId =
      req.query.grantId !== undefined ? Number(req.query.grantId) : null;

    // Resume from Last-Event-ID if the browser reconnects
    const lastEventIdHeader = req.headers["last-event-id"];
    let lastEventId = lastEventIdHeader
      ? parseInt(String(lastEventIdHeader), 10)
      : 0;

    // Set SSE response headers
    res.setHeader("Content-Type", "text/event-stream");
    res.setHeader("Cache-Control", "no-cache");
    res.setHeader("Connection", "keep-alive");
    res.setHeader("X-Accel-Buffering", "no");
    res.flushHeaders();

    // Initial connected heartbeat
    res.write("event: connected\ndata: {}\n\n");

    // Keepalive ping — prevents proxy/load-balancer idle timeouts
    const keepalive = setInterval(() => {
      if (!res.writableEnded) {
        res.write("event: ping\ndata: {}\n\n");
      }
    }, KEEPALIVE_INTERVAL_MS);

    // Poll for new Activity rows every 5 seconds
    const poll = setInterval(async () => {
      if (res.writableEnded) return;

      try {
        const where: Record<string, unknown> = {
          id: MoreThan(lastEventId),
        };
        if (grantId !== null) {
          where.entityId = grantId;
          where.entityType = "grant";
        }

        const events = await activityRepo.find({
          where,
          order: { id: "ASC" },
          take: 50,
        });

        for (const event of events) {
          // Only surface the streamed subset of activity types
          if (!STREAMED_TYPES.has(event.type)) {
            lastEventId = Math.max(lastEventId, event.id);
            continue;
          }

          const payload = JSON.stringify({
            grantId: event.entityId,
            type: event.type,
            actorAddress: event.actorAddress,
            timestamp: event.timestamp,
            data: event.data,
          });

          res.write(`id: ${event.id}\nevent: ${event.type}\ndata: ${payload}\n\n`);
          lastEventId = event.id;
        }
      } catch (err) {
        // Log but do not crash the stream — transient DB errors should not
        // terminate the client connection.
        const message = err instanceof Error ? err.message : String(err);
        console.error("[events-sse] poll error", message);
      }
    }, POLL_INTERVAL_MS);

    // Cleanup on client disconnect
    req.on("close", () => {
      clearInterval(keepalive);
      clearInterval(poll);
      decrementConnections(ip);
      metricsService.decrementSseConnections();
    });
  });

  return router;
};
