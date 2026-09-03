import { DataSource } from "typeorm";
import { RateLimitLog } from "../entities/RateLimitLog";
import { env } from "../config/env";
import { logger } from "../config/logger";
import { notificationService } from "./notification-service";

export class RateLimitAlertService {
  private lastAlertAtMs = 0;

  constructor(private readonly dataSource: DataSource) {}

  async checkOnce(): Promise<void> {
    const windowMinutes = env.rateLimitAlertWindowMinutes;
    const threshold = env.rateLimitAlertThreshold;
    const cooldownMs = env.rateLimitAlertCooldownMinutes * 60 * 1000;

    if (Date.now() - this.lastAlertAtMs < cooldownMs) return;

    const since = new Date(Date.now() - windowMinutes * 60 * 1000);
    const repo = this.dataSource.getRepository(RateLimitLog);

    const raw = await repo.createQueryBuilder("rl")
      .select("COUNT(*)", "hits")
      .where("rl.createdAt >= :since", { since })
      .getRawOne<{ hits: string }>();

    const hits = Number(raw?.hits ?? 0);
    if (hits < threshold) return;

    this.lastAlertAtMs = Date.now();

    const payload = {
      type: "rate_limit_spike",
      windowMinutes,
      hits,
      threshold,
      since: since.toISOString(),
    };

    logger.warn("Rate limit spike detected", payload);
    notificationService.broadcast("rate_limit_spike", payload);
  }
}

