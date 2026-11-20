import { Counter, Gauge, Histogram, Registry, collectDefaultMetrics } from "prom-client";

class MetricsService {
  private readonly registry = new Registry();

  private readonly httpRequestDurationMs = new Histogram({
    name: "stellargrant_http_request_duration_ms",
    help: "HTTP request duration in milliseconds",
    labelNames: ["method", "route", "status_code"],
    buckets: [5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000],
    registers: [this.registry],
  });

  private readonly httpRequestsTotal = new Counter({
    name: "stellargrant_http_requests_total",
    help: "Total number of HTTP requests",
    labelNames: ["method", "route", "status_code"],
    registers: [this.registry],
  });

  private readonly grantsCreatedTotal = new Counter({
    name: "stellargrant_grants_created_total",
    help: "Total number of grants created from on-chain sync",
    registers: [this.registry],
  });

  private readonly onchainReconciliationRunsTotal = new Counter({
    name: "stellargrant_onchain_reconciliation_runs_total",
    help: "Total reconciliation task runs grouped by status",
    labelNames: ["status"],
    registers: [this.registry],
  });

  private readonly onchainReconciliationGapsTotal = new Counter({
    name: "stellargrant_onchain_reconciliation_gaps_total",
    help: "Total number of reconciliation data gaps discovered",
    registers: [this.registry],
  });

  private readonly walletRateLimitHitsTotal = new Counter({
    name: "rate_limit_wallet_hits_total",
    help: "Total number of wallet-based rate limit hits",
    labelNames: ["endpoint"],
    registers: [this.registry],
  });

  private readonly sseConnectionsTotal = new Counter({
    name: "stellargrant_sse_connections_total",
    help: "Total number of SSE connections established",
    registers: [this.registry],
  });

  private readonly sseActiveConnections = new Gauge({
    name: "stellargrant_sse_active_connections",
    help: "Current number of open SSE connections",
    registers: [this.registry],
  });

  constructor() {
    collectDefaultMetrics({
      prefix: "stellargrant_",
      register: this.registry,
    });
  }

  observeHttpRequest(method: string, route: string, statusCode: number, durationMs: number): void {
    const labels = { method, route, status_code: String(statusCode) };
    this.httpRequestsTotal.inc(labels);
    this.httpRequestDurationMs.observe(labels, durationMs);
  }

  incrementGrantCreated(): void {
    this.grantsCreatedTotal.inc();
  }

  recordWalletRateLimitHit(endpoint: string): void {
    this.walletRateLimitHitsTotal.inc({ endpoint });
  }

  recordReconciliationRun(status: "success" | "failure", gapsFound = 0): void {
    this.onchainReconciliationRunsTotal.inc({ status });
    if (gapsFound > 0) {
      this.onchainReconciliationGapsTotal.inc(gapsFound);
    }
  }

  incrementSseConnections(): void {
    this.sseConnectionsTotal.inc();
    this.sseActiveConnections.inc();
  }

  decrementSseConnections(): void {
    this.sseActiveConnections.dec();
  }

  async getMetricsText(): Promise<string> {
    return this.registry.metrics();
  }

  getContentType(): string {
    return this.registry.contentType;
  }
}

export const metricsService = new MetricsService();
