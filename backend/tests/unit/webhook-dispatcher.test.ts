import { describe, it, expect, beforeEach, vi } from "vitest";
import { WebhookDispatcher, WebhookPayload } from "../../src/services/webhook-dispatcher";
import { WebhookSubscription, WebhookEventType } from "../../src/entities/WebhookSubscription";
import { WebhookDeliveryLog, WebhookDeliveryStatus } from "../../src/entities/WebhookDeliveryLog";
import { Repository, DataSource } from "typeorm";

describe("WebhookDispatcher", () => {
  let dispatcher: WebhookDispatcher;
  let subscriptionRepo: Repository<WebhookSubscription>;
  let deliveryLogRepo: Repository<WebhookDeliveryLog>;
  let mockDataSource: DataSource;

  beforeEach(() => {
    subscriptionRepo = {
      find: vi.fn(),
      findOne: vi.fn(),
      save: vi.fn(),
    } as any;

    deliveryLogRepo = {
      find: vi.fn(),
      findAndCount: vi.fn(),
      save: vi.fn(),
      create: vi.fn((data: any) => ({ ...data, id: 1 })),
    } as any;

    mockDataSource = {
      getRepository: vi.fn((entity: any) => {
        if (entity === WebhookSubscription) return subscriptionRepo;
        if (entity === WebhookDeliveryLog) return deliveryLogRepo;
        return {} as any;
      }),
    } as any;

    dispatcher = new WebhookDispatcher(mockDataSource);
  });

  describe("signPayload", () => {
    it("should generate a consistent HMAC signature", () => {
      const payload: WebhookPayload = {
        event: "grant.created",
        timestamp: "2024-01-01T00:00:00Z",
        data: { grantId: 1 },
      };

      const signature1 = WebhookDispatcher.signPayload(payload, "my-secret");
      const signature2 = WebhookDispatcher.signPayload(payload, "my-secret");

      expect(signature1).toBe(signature2);
      expect(signature1).toMatch(/^[a-f0-9]{64}$/);
    });

    it("should produce different signatures for different secrets", () => {
      const payload: WebhookPayload = {
        event: "grant.created",
        timestamp: "2024-01-01T00:00:00Z",
        data: { grantId: 1 },
      };

      const sig1 = WebhookDispatcher.signPayload(payload, "secret-a");
      const sig2 = WebhookDispatcher.signPayload(payload, "secret-b");

      expect(sig1).not.toBe(sig2);
    });
  });

  describe("verifySignature", () => {
    it("should verify a valid signature", () => {
      const payload: WebhookPayload = {
        event: "grant.created",
        timestamp: "2024-01-01T00:00:00Z",
        data: { grantId: 1 },
      };

      const signature = WebhookDispatcher.signPayload(payload, "my-secret");
      const isValid = WebhookDispatcher.verifySignature(payload, "my-secret", signature);

      expect(isValid).toBe(true);
    });

    it("should reject an invalid signature", () => {
      const payload: WebhookPayload = {
        event: "grant.created",
        timestamp: "2024-01-01T00:00:00Z",
        data: { grantId: 1 },
      };

      const isValid = WebhookDispatcher.verifySignature(payload, "my-secret", "invalid-signature");

      expect(isValid).toBe(false);
    });
  });

  describe("dispatch", () => {
    it("should dispatch to matching subscriptions", async () => {
      const subscription: WebhookSubscription = {
        id: 1,
        targetUrl: "https://example.com/webhook",
        secretKey: "test-secret",
        events: [WebhookEventType.GRANT_CREATED],
        isActive: true,
        failureCount: 0,
        maxRetries: 5,
        communityId: null,
        ownerAddress: null,
        createdBy: 1,
        user: {} as any,
        createdAt: new Date(),
        updatedAt: new Date(),
      };

      vi.spyOn(subscriptionRepo, "find").mockResolvedValue([subscription]);
      vi.spyOn(deliveryLogRepo, "save").mockResolvedValue({ id: 1 } as any);

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        text: vi.fn().mockResolvedValue("OK"),
      });

      await dispatcher.dispatch(WebhookEventType.GRANT_CREATED, { grantId: 1 });

      expect(subscriptionRepo.find).toHaveBeenCalledWith({ where: { isActive: true } });
      expect(global.fetch).toHaveBeenCalledTimes(1);

      const fetchCall = (global.fetch as any).mock.calls[0];
      expect(fetchCall[0]).toBe("https://example.com/webhook");

      const requestInit = fetchCall[1];
      expect(requestInit.method).toBe("POST");
      expect(requestInit.headers["X-Webhook-Event"]).toBe("grant.created");
      expect(requestInit.headers["X-Webhook-Signature"]).toMatch(/^sha256=/);
    });

    it("should not dispatch when no matching subscriptions", async () => {
      vi.spyOn(subscriptionRepo, "find").mockResolvedValue([]);

      global.fetch = vi.fn();

      await dispatcher.dispatch(WebhookEventType.GRANT_CREATED, { grantId: 1 });

      expect(global.fetch).not.toHaveBeenCalled();
    });

    it("should dispatch to wildcard subscriptions", async () => {
      const subscription: WebhookSubscription = {
        id: 2,
        targetUrl: "https://example.com/all",
        secretKey: "wildcard-secret",
        events: [WebhookEventType.ALL],
        isActive: true,
        failureCount: 0,
        maxRetries: 5,
        communityId: null,
        ownerAddress: null,
        createdBy: 1,
        user: {} as any,
        createdAt: new Date(),
        updatedAt: new Date(),
      };

      vi.spyOn(subscriptionRepo, "find").mockResolvedValue([subscription]);
      vi.spyOn(deliveryLogRepo, "save").mockResolvedValue({ id: 2 } as any);

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        text: vi.fn().mockResolvedValue("OK"),
      });

      await dispatcher.dispatch(WebhookEventType.MILESTONE_APPROVED, { milestoneIdx: 0 });

      expect(global.fetch).toHaveBeenCalledTimes(1);
    });

    it("should handle failed deliveries and schedule retries", async () => {
      const subscription: WebhookSubscription = {
        id: 3,
        targetUrl: "https://example.com/fail",
        secretKey: "fail-secret",
        events: [WebhookEventType.GRANT_CREATED],
        isActive: true,
        failureCount: 0,
        maxRetries: 5,
        communityId: null,
        ownerAddress: null,
        createdBy: 1,
        user: {} as any,
        createdAt: new Date(),
        updatedAt: new Date(),
      };

      vi.spyOn(subscriptionRepo, "find").mockResolvedValue([subscription]);
      vi.spyOn(subscriptionRepo, "save").mockResolvedValue(subscription);

      const savedLog = { id: 3, attemptCount: 0, status: WebhookDeliveryStatus.PENDING, nextRetryAt: null as string | null };
      vi.spyOn(deliveryLogRepo, "create").mockReturnValue(savedLog as any);
      vi.spyOn(deliveryLogRepo, "save").mockResolvedValue(savedLog as any);

      global.fetch = vi.fn().mockRejectedValue(new Error("Network error"));

      await dispatcher.dispatch(WebhookEventType.GRANT_CREATED, { grantId: 1 });

      expect(deliveryLogRepo.save).toHaveBeenCalled();
      expect(savedLog.status).toBe(WebhookDeliveryStatus.RETRYING);
      expect(typeof savedLog.nextRetryAt).toBe("string");
      expect(savedLog.nextRetryAt).not.toBeNull();
      expect(new Date(savedLog.nextRetryAt!)).toBeInstanceOf(Date);
    });

    it("should disable subscription after too many failures", async () => {
      const subscription: WebhookSubscription = {
        id: 4,
        targetUrl: "https://example.com/fail",
        secretKey: "fail-secret",
        events: [WebhookEventType.GRANT_CREATED],
        isActive: true,
        failureCount: 9,
        maxRetries: 1,
        communityId: null,
        ownerAddress: null,
        createdBy: 1,
        user: {} as any,
        createdAt: new Date(),
        updatedAt: new Date(),
      };

      vi.spyOn(subscriptionRepo, "find").mockResolvedValue([subscription]);
      vi.spyOn(subscriptionRepo, "save").mockImplementation((sub: any) => sub);

      const savedLog = { id: 4, attemptCount: 1, status: WebhookDeliveryStatus.PENDING };
      vi.spyOn(deliveryLogRepo, "create").mockReturnValue(savedLog as any);
      vi.spyOn(deliveryLogRepo, "save").mockImplementation((log: any) => log);

      global.fetch = vi.fn().mockRejectedValue(new Error("Network error"));

      await dispatcher.dispatch(WebhookEventType.GRANT_CREATED, { grantId: 1 });

      expect(subscription.isActive).toBe(false);
      expect(subscription.failureCount).toBe(10);
    });
  });
});
