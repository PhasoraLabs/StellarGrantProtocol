import { Keypair } from "@stellar/stellar-sdk";
import request from "supertest";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import { SignatureService } from "../../src/services/signature-service";
import { Activity } from "../../src/entities/Activity";
import { ReconciliationCheckpoint } from "../../src/entities/ReconciliationCheckpoint";
import { env } from "../../src/config/env";

describe("POST /events/replay", () => {
  let dataSource: DataSource;
  let app: ReturnType<typeof createApp>;
  const sorobanClient = new MockSorobanContractClient();
  const signatureService = new SignatureService();
  const adminKeypair = Keypair.random();
  const adminAddress = adminKeypair.publicKey();
  const path = "/events/replay";

  beforeAll(async () => {
    env.adminAddresses = [adminAddress];
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();
    app = createApp(dataSource, sorobanClient);
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  const adminHeaders = (overrides: Record<string, string> = {}) => {
    const nonce = Math.random().toString(36).substring(7);
    const timestamp = Date.now();
    const message = signatureService.buildAdminIntentMessage({
      address: adminAddress,
      nonce,
      timestamp,
      action: `POST:${path}`,
    });

    return {
      "x-admin-address": adminAddress,
      "x-admin-signature": adminKeypair.sign(Buffer.from(message, "utf8")).toString("base64"),
      "x-admin-nonce": nonce,
      "x-admin-timestamp": String(timestamp),
      ...overrides,
    };
  };

  it("rejects requests without admin authentication", async () => {
    const response = await request(app).post(path).send({ fromLedger: 900 });

    expect(response.status).toBe(401);
  });

  it("rejects non-admin addresses", async () => {
    const headers = adminHeaders({
      "x-admin-address": Keypair.random().publicKey(),
    });

    const response = await request(app).post(path).set(headers).send({ fromLedger: 900 });

    expect(response.status).toBe(403);
  });

  it("validates the requested ledger range", async () => {
    const negative = await request(app)
      .post(path)
      .set(adminHeaders())
      .send({ fromLedger: -1 });
    expect(negative.status).toBe(400);

    const inverted = await request(app)
      .post(path)
      .set(adminHeaders())
      .send({ fromLedger: 950, toLedger: 900 });
    expect(inverted.status).toBe(400);
  });

  it("re-indexes events from the requested ledger without moving the checkpoint", async () => {
    const response = await request(app)
      .post(path)
      .set(adminHeaders())
      .send({ fromLedger: 900, toLedger: 1000 });

    expect(response.status).toBe(200);
    expect(response.body.ok).toBe(true);
    expect(response.body.data).toMatchObject({
      fromLedger: 900,
      toLedger: 1000,
      eventsFound: 2,
      gapsFound: 2,
      gapsFilled: 2,
    });

    const activities = await dataSource.getRepository(Activity).find();
    const replayed = activities.filter((activity) => activity.data?.reconciled === true);
    expect(replayed).toHaveLength(2);
    expect(replayed.map((activity) => activity.data?.ledger).sort()).toEqual([900, 901]);

    // Replay must not advance the normal reconciliation checkpoint.
    const checkpoints = await dataSource.getRepository(ReconciliationCheckpoint).find();
    expect(checkpoints).toHaveLength(0);
  });

  it("returns empty results for a ledger range with no events", async () => {
    const response = await request(app)
      .post(path)
      .set(adminHeaders())
      .send({ fromLedger: 800, toLedger: 899 });

    expect(response.status).toBe(200);
    expect(response.body.data).toMatchObject({
      fromLedger: 800,
      toLedger: 899,
      eventsFound: 0,
      gapsFound: 0,
      gapsFilled: 0,
    });
  });
});
