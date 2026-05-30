import { Keypair } from "@stellar/stellar-sdk";
import request from "supertest";
import { beforeAll, afterAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { SignatureService } from "../../src/services/signature-service";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";

describe("API e2e", () => {
  let dataSource: DataSource;
  const sorobanClient = new MockSorobanContractClient();
  const signatureService = new SignatureService();

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  it("fetches grants from mock Soroban and caches them", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/grants");

    expect(response.status).toBe(200);
    expect(response.body.data).toHaveLength(4);
    expect(response.body.data[0].title).toBe("Open Source Grants Q2");
  });

  it("returns a specific grant by id", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/grants/1");

    expect(response.status).toBe(200);
    expect(response.body.data.id).toBe(1);
  });

  it("accepts signed milestone proof", async () => {
    const app = createApp(dataSource, sorobanClient);
    const keypair = Keypair.random();
    const timestamp = Date.now();
    const payload = {
      grantId: 1,
      milestoneIdx: 0,
      proofCid: "bafybeihk-example-proof",
      submittedBy: keypair.publicKey(),
      nonce: "nonce-123456",
      timestamp,
    };

    const message = signatureService.buildIntentMessage({ ...payload });
    const signature = keypair.sign(Buffer.from(message, "utf8")).toString("base64");

    const response = await request(app).post("/milestone_proof").send({
      ...payload,
      signature,
    });

    expect(response.status).toBe(201);
    expect(response.body.data.grantId).toBe(1);
    expect(response.body.data.milestoneIdx).toBe(0);
  });

  it("rejects milestone proof with invalid signature", async () => {
    const app = createApp(dataSource, sorobanClient);
    const keypair = Keypair.random();

    const response = await request(app).post("/milestone_proof").send({
      grantId: 1,
      milestoneIdx: 1,
      proofCid: "bafybeihk-example-proof-2",
      submittedBy: keypair.publicKey(),
      nonce: "nonce-abcdef12",
      timestamp: Date.now(),
      signature: Buffer.from("x".repeat(64)).toString("base64"),
    });

    expect(response.status).toBe(401);
  });
});
