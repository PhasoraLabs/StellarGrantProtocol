import { Keypair } from "@stellar/stellar-sdk";
import request from "supertest";
import { beforeAll, afterAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import { SignatureService } from "../../src/services/signature-service";
import { AuditLog } from "../../src/entities/AuditLog";
import { Contributor } from "../../src/entities/Contributor";
import { env } from "../../src/config/env";

describe("Admin API e2e", () => {
  let dataSource: DataSource;
  const sorobanClient = new MockSorobanContractClient();
  const signatureService = new SignatureService();
  const adminKeypair = Keypair.random();
  const adminAddress = adminKeypair.publicKey();

  beforeAll(async () => {
    // Override env directly since it's already loaded
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

  it("rejects request without admin headers", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).post("/admin/sync/1");
    expect(response.status).toBe(401);
  });

  it("rejects request from non-admin address", async () => {
    const app = createApp(dataSource, sorobanClient);
    const otherKeypair = Keypair.random();
    const headers = getAdminHeaders("POST", "/admin/sync/1");
    headers["x-admin-address"] = otherKeypair.publicKey(); // Signature will be invalid now too

    const response = await request(app).post("/admin/sync/1").set(headers);
    expect(response.status).toBe(403);
  });

  it("allows admin to sync a grant", async () => {
    const app = createApp(dataSource, sorobanClient);
    const path = "/admin/sync/1";
    const headers = getAdminHeaders("POST", path);

    const response = await request(app).post(path).set(headers);

    expect(response.status).toBe(200);
    expect(response.body.ok).toBe(true);

    // Verify AuditLog
    const auditLogRepo = dataSource.getRepository(AuditLog);
    const log = await auditLogRepo.findOne({ where: { action: "SYNC_GRANT" } });
    expect(log).toBeDefined();
    expect(log?.adminAddress).toBe(adminAddress);
  });

  it("allows admin to blacklist a user", async () => {
    const app = createApp(dataSource, sorobanClient);
    const targetUser = "GAV3TIZZ7DRCCMUVKZRQXELRTJFMXQT4XJFNV5BYMNOFXWXZA5MGDVEV";
    const path = `/admin/users/${targetUser}/blacklist`;
    const headers = getAdminHeaders("PATCH", path);

    const response = await request(app)
      .patch(path)
      .set(headers)
      .send({ blacklist: true });

    expect(response.status).toBe(200);
    expect(response.body.isBlacklisted).toBe(true);

    // Verify DB state
    const contributorRepo = dataSource.getRepository(Contributor);
    const contributor = await contributorRepo.findOne({ where: { address: targetUser } });
    expect(contributor?.isBlacklisted).toBe(true);

    // Verify AuditLog
    const auditLogRepo = dataSource.getRepository(AuditLog);
    const log = await auditLogRepo.findOne({ where: { action: "BLACKLIST_USER" } });
    expect(log).toBeDefined();
    expect(log?.target).toBe(`user:${targetUser}`);
  });
});
