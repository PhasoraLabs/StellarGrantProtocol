import { Keypair } from "@stellar/stellar-sdk";
import request from "supertest";
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { Grant } from "../../src/entities/Grant";
import { MilestoneProof } from "../../src/entities/MilestoneProof";
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

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches grants from mock Soroban and caches them", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/grants");

    expect(response.status).toBe(200);
    expect(response.body.data).toHaveLength(4);
    expect(response.body.data[0].title).toBe("Open Source Grants Q2");
  });

  it.each([
    "limit=not-a-number",
    "limit=0",
    "limit=101",
    "page=not-a-number",
    "page=0",
    "page=1.5",
  ])("rejects invalid grant pagination query %s", async (query) => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get(`/grants?${query}`);

    expect(response.status).toBe(400);
    expect(response.body.error).toBe("Validation failed");
  });

  it("uses validated page and limit values for grant pagination", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/grants?page=2&limit=1");

    expect(response.status).toBe(200);
    expect(response.body.data).toHaveLength(1);
    expect(response.body.meta.page).toBe(2);
    expect(response.body.meta.limit).toBe(1);
  });

  it("caches stats responses and sends a short cache lifetime", async () => {
    const app = createApp(dataSource, sorobanClient);
    const grantRepo = dataSource.getRepository(Grant);
    const proofRepo = dataSource.getRepository(MilestoneProof);
    const grantCount = vi.spyOn(grantRepo, "count");
    const proofCount = vi.spyOn(proofRepo, "count");
    const grantFind = vi.spyOn(grantRepo, "find");
    const now = vi.spyOn(Date, "now").mockReturnValue(1_000);

    const firstResponse = await request(app).get("/stats");
    now.mockReturnValue(30_999);
    const cachedResponse = await request(app).get("/stats");

    expect(firstResponse.status).toBe(200);
    expect(firstResponse.headers["cache-control"]).toBe("public, max-age=30");
    expect(cachedResponse.status).toBe(200);
    expect(cachedResponse.headers["cache-control"]).toBe("public, max-age=30");
    expect(cachedResponse.body).toEqual(firstResponse.body);
    expect(grantCount).toHaveBeenCalledOnce();
    expect(proofCount).toHaveBeenCalledOnce();
    expect(grantFind).toHaveBeenCalledOnce();

    now.mockReturnValue(31_001);
    const expiredResponse = await request(app).get("/stats");

    expect(expiredResponse.status).toBe(200);
    expect(grantCount).toHaveBeenCalledTimes(2);
    expect(proofCount).toHaveBeenCalledTimes(2);
    expect(grantFind).toHaveBeenCalledTimes(2);
  });

  it("generates and returns a request ID when none is provided", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/health");

    expect(response.headers["x-request-id"]).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i,
    );
  });

  it("propagates a valid incoming request ID", async () => {
    const app = createApp(dataSource, sorobanClient);
    const requestId = "client-request-123";
    const response = await request(app)
      .get("/health")
      .set("X-Request-Id", requestId);

    expect(response.headers["x-request-id"]).toBe(requestId);
  });

  it("replaces invalid incoming request IDs", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app)
      .get("/health")
      .set("X-Request-Id", "invalid request id");

    expect(response.headers["x-request-id"]).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i,
    );
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
