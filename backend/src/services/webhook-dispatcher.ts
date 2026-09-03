import { Repository, DataSource } from "typeorm";
import { WebhookSubscription, WebhookEventType } from "../entities/WebhookSubscription";
import { WebhookDeliveryLog, WebhookDeliveryStatus } from "../entities/WebhookDeliveryLog";
import { createHmac } from "node:crypto";

/**
 * Webhook event payload structure
 */
export interface WebhookPayload {
  event: string;
  timestamp: string;
  data: Record<string, any>;
}

/**
 * Retry configuration with exponential backoff
 */
const RETRY_DELAYS_MS = [
  5000,    // 5 seconds
  15000,   // 15 seconds
  60000,   // 1 minute
  300000,  // 5 minutes
  900000,  // 15 minutes
];

export class WebhookDispatcher {
  private subscriptionRepo: Repository<WebhookSubscription>;
  private deliveryLogRepo: Repository<WebhookDeliveryLog>;
  private retryTimer: ReturnType<typeof setInterval> | null = null;

  constructor(private readonly dataSource: DataSource) {
    this.subscriptionRepo = this.dataSource.getRepository(WebhookSubscription);
    this.deliveryLogRepo = this.dataSource.getRepository(WebhookDeliveryLog);
    this.startRetryQueue();
  }

  /**
   * Generate HMAC-SHA256 signature for webhook payload
   */
  static signPayload(payload: WebhookPayload, secret: string): string {
    const body = JSON.stringify(payload);
    return createHmac("sha256", secret).update(body).digest("hex");
  }

  /**
   * Verify webhook signature
   */
  static verifySignature(payload: WebhookPayload, secret: string, signature: string): boolean {
    try {
      return createHmac("sha256", secret)
        .update(JSON.stringify(payload))
        .digest("hex") === signature;
    } catch {
      return false;
    }
  }

  /**
   * Dispatch an event to all matching webhook subscriptions
   */
  async dispatch(event: WebhookEventType, data: Record<string, any>): Promise<void> {
    const subscriptions = await this.subscriptionRepo.find({
      where: { isActive: true },
    });

    // Filter subscriptions that are interested in this event
    const matching = subscriptions.filter((subscription: WebhookSubscription) =>
      subscription.events.includes(event) || subscription.events.includes(WebhookEventType.ALL),
    );

    if (matching.length === 0) {
      return;
    }

    const payload: WebhookPayload = {
      event,
      timestamp: new Date().toISOString(),
      data,
    };

    const deliveryPromises = matching.map((subscription: WebhookSubscription) =>
      this.deliver(subscription, payload).catch((err: any) => {
        console.error(`Webhook delivery failed for subscription ${subscription.id}:`, err);
      }),
    );

    await Promise.allSettled(deliveryPromises);
  }

  /**
   * Deliver a payload to a specific subscription
   */
  private async deliver(
    subscription: WebhookSubscription,
    payload: WebhookPayload,
  ): Promise<WebhookDeliveryLog> {
    const signature = WebhookDispatcher.signPayload(payload, subscription.secretKey);

    const log = this.deliveryLogRepo.create({
      subscriptionId: subscription.id,
      eventType: payload.event,
      payload,
      payloadSignature: signature,
      status: WebhookDeliveryStatus.PENDING,
      attemptCount: 0,
    });

    await this.deliveryLogRepo.save(log);
    await this.attemptDelivery(log, subscription, payload, signature);

    return log;
  }

  /**
   * Attempt to deliver a webhook
   */
  private async attemptDelivery(
    log: WebhookDeliveryLog,
    subscription: WebhookSubscription,
    payload: WebhookPayload,
    signature: string,
  ): Promise<void> {
    log.attemptCount += 1;

    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 30000); // 30s timeout

      const response = await fetch(subscription.targetUrl, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Webhook-Signature": `sha256=${signature}`,
          "X-Webhook-Event": payload.event,
          "X-Webhook-Id": String(log.id),
          "X-Webhook-Timestamp": payload.timestamp,
          "User-Agent": "StellarGrant-Webhook/1.0",
        },
        body: JSON.stringify(payload),
        signal: controller.signal,
      });

      clearTimeout(timeout);

      const responseBody = await response.text();
      log.httpStatusCode = response.status;
      log.responseBody = responseBody;

      if (response.ok) {
        log.status = WebhookDeliveryStatus.DELIVERED;

        // Reset failure count on successful delivery
        if (subscription.failureCount > 0) {
          subscription.failureCount = 0;
          await this.subscriptionRepo.save(subscription);
        }
      } else {
        log.status = WebhookDeliveryStatus.FAILED;
        log.errorMessage = `HTTP ${response.status}: ${responseBody.slice(0, 200)}`;
        await this.scheduleRetry(log, subscription);
      }
    } catch (error) {
      log.status = WebhookDeliveryStatus.FAILED;
      log.errorMessage = error instanceof Error ? error.message : "Network error";
      await this.scheduleRetry(log, subscription);
    }

    await this.deliveryLogRepo.save(log);
  }

  /**
   * Schedule a retry for a failed delivery
   */
  private async scheduleRetry(
    log: WebhookDeliveryLog,
    subscription: WebhookSubscription,
  ): Promise<void> {
    if (log.attemptCount >= Math.min(subscription.maxRetries, RETRY_DELAYS_MS.length)) {
      log.status = WebhookDeliveryStatus.EXHAUSTED;

      // Track consecutive failures on subscription
      subscription.failureCount += 1;

      // Auto-disable subscription after too many consecutive failures
      if (subscription.failureCount >= 10) {
        subscription.isActive = false;
      }

      await this.subscriptionRepo.save(subscription);
      return;
    }

    const delay = RETRY_DELAYS_MS[log.attemptCount - 1] ?? RETRY_DELAYS_MS.at(-1)!;
    log.nextRetryAt = new Date(Date.now() + delay).toISOString();
    log.status = WebhookDeliveryStatus.RETRYING;
  }

  /**
   * Start the background retry queue processor
   */
  private startRetryQueue(): void {
    if (this.retryTimer) {
      return;
    }

    // Process retries every 30 seconds
    this.retryTimer = setInterval(async () => {
      try {
        await this.processRetries();
      } catch (error) {
        console.error("Webhook retry queue error:", error);
      }
    }, 30000);

    // Prevent unhandled rejections in Node.js
    if (this.retryTimer.unref) {
      this.retryTimer.unref();
    }
  }

  /**
   * Process pending retries
   */
  private async processRetries(): Promise<void> {
    const pendingRetries = await this.deliveryLogRepo.find({
      where: {
        status: WebhookDeliveryStatus.RETRYING,
        nextRetryAt: { $lte: new Date().toISOString() } as any,
      },
      relations: ["subscription"],
      take: 50, // Batch size
    });

    for (const log of pendingRetries) {
      if (!log.subscription || !log.subscription.isActive) {
        log.status = WebhookDeliveryStatus.EXHAUSTED;
        await this.deliveryLogRepo.save(log);
        continue;
      }

      const payload: WebhookPayload = log.payload as WebhookPayload;
      const signature = WebhookDispatcher.signPayload(payload, log.subscription.secretKey);

      try {
        await this.attemptDelivery(log, log.subscription, payload, signature);
      } catch (error) {
        console.error(`Retry failed for delivery ${log.id}:`, error);
      }
    }
  }

  /**
   * Get delivery logs for a subscription
   */
  async getDeliveryLogs(
    subscriptionId: number,
    limit = 50,
    offset = 0,
  ): Promise<{ logs: WebhookDeliveryLog[]; total: number }> {
    const [logs, total] = await this.deliveryLogRepo.findAndCount({
      where: { subscriptionId },
      order: { createdAt: "DESC" },
      take: limit,
      skip: offset,
    });

    return { logs, total };
  }

  /**
   * Stop the retry queue (for graceful shutdown)
   */
  stop(): void {
    if (this.retryTimer) {
      clearInterval(this.retryTimer);
      this.retryTimer = null;
    }
  }
}
