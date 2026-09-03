import { Keypair } from "@stellar/stellar-sdk";
import request from "supertest";
import { beforeAll, afterAll, describe, expect, it, vi } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import { ReconciliationService } from "../../src/services/reconciliation-service";
import { GrantSyncService } from "../../src/services/grant-sync-service";
import { Activity } from "../../src/entities/Activity";
import { ReconciliationCheckpoint } from "../../src/entities/ReconciliationCheckpoint";
import { SignatureService } from "../../src/services/signature-service";
import { env } from "../../src/config/env";
import { AuditLog } from "../../src/entities/AuditLog";

describe("Reconciliation e2e", () => {
    let dataSource: DataSource;
    const sorobanClient = new MockSorobanContractClient();
    const signatureService = new SignatureService();
    const adminKeypair = Keypair.random();
    const adminAddress = adminKeypair.publicKey();

    beforeAll(async () => {
        env.adminAddresses = [adminAddress];
        dataSource = buildDataSource("sqljs://memory");
        await dataSource.initialize();
    });

    afterAll(async () => {
        if (dataSource?.isInitialized) {
            await dataSource.destroy();
        }
    });

    const getAdminHeaders = (method: string, path: string) => {
        const nonce = Math.random().toString(36).substring(7);
        const timestamp = Date.now();
        const action = `${method}:${path}`;
        const message = signatureService.buildAdminIntentMessage({
            address: adminAddress,
            nonce,
            timestamp,
            action,
        });
        const signature = adminKeypair.sign(Buffer.from(message, "utf8")).toString("base64");
        return {
            "x-admin-address": adminAddress,
            "x-admin-signature": signature,
            "x-admin-nonce": nonce,
            "x-admin-timestamp": timestamp.toString(),
        };
    };

    describe("ReconciliationService unit", () => {
        it("creates a checkpoint on first run", async () => {
            const grantSyncService = new GrantSyncService(dataSource, sorobanClient);
            const service = new ReconciliationService(dataSource, sorobanClient, grantSyncService);

            const result = await service.run();

            expect(result).toBeDefined();
            expect(result.eventsFound).toBeGreaterThanOrEqual(0);
            expect(result.errors).toHaveLength(0);

            const checkpointRepo = dataSource.getRepository(ReconciliationCheckpoint);
            const checkpoint = await checkpointRepo.findOne({ where: { name: "main" } });
            expect(checkpoint).toBeDefined();
            expect(checkpoint!.lastLedger).toBeGreaterThan(0);
        });

        it("fills missing activity records for on-chain events", async () => {
            const grantSyncService = new GrantSyncService(dataSource, sorobanClient);
            const service = new ReconciliationService(dataSource, sorobanClient, grantSyncService);

            // Reset checkpoint so the run scans a fresh range
            const checkpointRepo = dataSource.getRepository(ReconciliationCheckpoint);
            await checkpointRepo.delete({ name: "main" });

            // Clear existing activities so gaps are detectable
            const activityRepo = dataSource.getRepository(Activity);
            await activityRepo.clear();

            const result = await service.run();

            expect(result.gapsFound).toBeGreaterThan(0);
            expect(result.gapsFilled).toBe(result.gapsFound);

            // Activities should now exist with reconciled flag
            const activities = await activityRepo.find();
            const reconciledActivities = activities.filter((a) => a.data?.reconciled === true);
            expect(reconciledActivities.length).toBeGreaterThan(0);
        });

        it("does not create duplicate activity records on a second run", async () => {
            const grantSyncService = new GrantSyncService(dataSource, sorobanClient);
            const service = new ReconciliationService(dataSource, sorobanClient, grantSyncService);

            const activityRepo = dataSource.getRepository(Activity);
            const countBefore = await activityRepo.count();

            // Run again — checkpoint is already advanced, so no new ledgers to scan
            const result = await service.run();

            const countAfter = await activityRepo.count();
            expect(countAfter).toBe(countBefore);
            expect(result.gapsFound).toBe(0);
        });

        it("runSafe returns null on error without throwing", async () => {
            const grantSyncService = new GrantSyncService(dataSource, sorobanClient);
            const service = new ReconciliationService(dataSource, sorobanClient, grantSyncService);

            // Force an error by making getLatestLedger throw
            vi.spyOn(sorobanClient, "getLatestLedger").mockRejectedValueOnce(new Error("network error"));

            const result = await service.runSafe();
            expect(result).toBeNull();

            vi.restoreAllMocks();
        });

        it("start and stop manage the scheduler without throwing", () => {
            const grantSyncService = new GrantSyncService(dataSource, sorobanClient);
            const service = new ReconciliationService(dataSource, sorobanClient, grantSyncService);

            service.start(999999); // very long interval so it doesn't fire
            service.start(999999); // calling start twice should be a no-op
            service.stop();
            service.stop(); // calling stop twice should be a no-op
        });
    });

    describe("POST /admin/reconcile", () => {
        it("rejects unauthenticated requests", async () => {
            const app = createApp(dataSource, sorobanClient);
            const res = await request(app).post("/admin/reconcile");
            expect(res.status).toBe(401);
        });

        it("triggers a reconciliation run and returns a result", async () => {
            const app = createApp(dataSource, sorobanClient);
            const path = "/admin/reconcile";
            const headers = getAdminHeaders("POST", path);

            const res = await request(app).post(path).set(headers);

            expect(res.status).toBe(200);
            expect(res.body.ok).toBe(true);
            expect(res.body.result).toBeDefined();
            expect(typeof res.body.result.eventsFound).toBe("number");
            expect(typeof res.body.result.gapsFound).toBe("number");
            expect(typeof res.body.result.gapsFilled).toBe("number");
            expect(typeof res.body.result.durationMs).toBe("number");

            // Verify audit log was written
            const auditLogRepo = dataSource.getRepository(AuditLog);
            const log = await auditLogRepo.findOne({ where: { action: "TRIGGER_RECONCILIATION" } });
            expect(log).toBeDefined();
        });
    });
});
