import { EmailService, EmailPayload } from "./email-service";
import { logger } from "../config/logger";

interface QueuedNotification {
  payload: EmailPayload;
  attempts: number;
}

export class NotificationQueue {
  private queue: QueuedNotification[] = [];
  private processing = false;
  private readonly maxAttempts = 3;
  private timer: ReturnType<typeof setInterval> | null = null;

  constructor(private readonly emailService: EmailService) {}

  enqueue(payload: EmailPayload): void {
    this.queue.push({ payload, attempts: 0 });
  }

  start(intervalMs = 5000): void {
    if (this.timer) return;
    this.timer = setInterval(() => this.flush(), intervalMs);
  }

  stop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  private async flush(): Promise<void> {
    if (this.processing || this.queue.length === 0) return;
    this.processing = true;

    const batch = this.queue.splice(0, 10);
    for (const item of batch) {
      try {
        await this.emailService.send(item.payload);
      } catch {
        item.attempts += 1;
        if (item.attempts < this.maxAttempts) {
          this.queue.push(item);
        } else {
          logger.warn("Dropping notification after max retries", {
            event: item.payload.event,
            to: item.payload.to,
          });
        }
      }
    }

    this.processing = false;
  }
}
