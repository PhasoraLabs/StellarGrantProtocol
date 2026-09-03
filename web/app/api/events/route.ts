/**
 * Events Stream API Route
 *
 * Server-Sent Events endpoint that streams decoded StellarGrants contract events
 * to the browser. Polls Stellar RPC for new events and forwards them to connected
 * clients. Sends keepalive pings every 25 s to prevent proxy timeouts.
 *
 * GET /api/events?grantId=42
 */

import { NextRequest } from "next/server";
import { fetchContractEvents, ContractEvent } from "@/lib/stellar/events";

const POLL_INTERVAL_MS = parseInt(process.env.SSE_POLL_INTERVAL_MS ?? "5000", 10);
const KEEPALIVE_INTERVAL_MS = 25_000;
const MAX_BACKOFF_MS = 30_000;

function sseEvent(event: string, data: unknown): string {
  return `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;
}

function ssePing(): string {
  return `event: ping\ndata: {}\n\n`;
}

export async function GET(request: NextRequest): Promise<Response> {
  const { searchParams } = new URL(request.url);
  const grantId = searchParams.get("grantId") ?? "";

  let lastLedger = 0;
  let backoffMs = POLL_INTERVAL_MS;

  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder();

      const send = (chunk: string) => {
        try {
          controller.enqueue(encoder.encode(chunk));
        } catch {
          // Client already disconnected — ignore write errors.
        }
      };

      let pollTimer: ReturnType<typeof setTimeout> | null = null;
      let keepaliveTimer: ReturnType<typeof setInterval> | null = null;
      let closed = false;

      const cleanup = () => {
        closed = true;
        if (pollTimer) clearTimeout(pollTimer);
        if (keepaliveTimer) clearInterval(keepaliveTimer);
      };

      // Send keepalive pings on a fixed interval independent of polling.
      keepaliveTimer = setInterval(() => {
        if (closed) return;
        send(ssePing());
      }, KEEPALIVE_INTERVAL_MS);

      const poll = async () => {
        if (closed || request.signal.aborted) {
          cleanup();
          try { controller.close(); } catch { /* already closed */ }
          return;
        }

        try {
          const events: ContractEvent[] = await fetchContractEvents(grantId);

          // Only forward events newer than the last seen ledger.
          const newEvents = events.filter((e) => e.ledger > lastLedger);

          for (const event of newEvents) {
            send(sseEvent("contract_event", event));
            if (event.ledger > lastLedger) lastLedger = event.ledger;
          }

          // Reset backoff on success.
          backoffMs = POLL_INTERVAL_MS;
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          send(sseEvent("error", { message }));
          // Exponential backoff: double delay up to MAX_BACKOFF_MS.
          backoffMs = Math.min(backoffMs * 2, MAX_BACKOFF_MS);
        }

        if (!closed && !request.signal.aborted) {
          pollTimer = setTimeout(poll, backoffMs);
        } else {
          cleanup();
          try { controller.close(); } catch { /* already closed */ }
        }
      };

      // Clean up when the client disconnects.
      request.signal.addEventListener("abort", () => {
        cleanup();
        try { controller.close(); } catch { /* already closed */ }
      });

      // Kick off first poll immediately.
      await poll();
    },
  });

  return new Response(stream, {
    status: 200,
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache, no-transform",
      "Connection": "keep-alive",
      "X-Accel-Buffering": "no",
    },
  });
}
