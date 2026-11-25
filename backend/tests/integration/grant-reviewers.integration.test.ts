import { randomUUID } from "node:crypto";
import { Keypair } from "@stellar/stellar-sdk";
import request from "supertest";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { Grant } from "../../src/entities/Grant";
import { GrantReviewer } from "../../src/entities/GrantReviewer";
import { SignatureService } from "../../src/services/signature-service";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";

const signatureService = new SignatureService();

describe("grant-reviewers routes", () => {
  let dataSource: DataSource;
  let app: ReturnType<typeof createApp>;

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();
    app = createApp(dataSource, new MockSorobanContractClient());
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  const createGrant = async (ownerAddress: string, recipientAddress?: string) =>
    dataSource.getRepository(Grant).save({
      title: "Grant reviewer integration test",
      status: "active",
      owner: ownerAddress,
      recipient: recipientAddress ?? ownerAddress,
      totalAmount: "500",
      isDraft: false,
    });

  const signAddReviewerPayload = (
    signer: Keypair,
    grantId: number,
    reviewerStellarAddress: string,
    overrides: {
      address?: string;
      nonce?: string;
      timestamp?: number;
      signature?: string;
    } = {},
  ) => {
    const address = overrides.address ?? signer.publicKey();
    const nonce = overrides.nonce ?? randomUUID();
    const timestamp = overrides.timestamp ?? Date.now();
    const message = signatureService.buildAddReviewerIntentMessage({
      grantId,
      reviewerStellarAddress,
      address,
      nonce,
      timestamp,
    });
    const signature =
      overrides.signature ??
      signer.sign(Buffer.from(message, "utf8")).toString("base64");

    return {
      grantId,
      reviewerStellarAddress,
      address,
      nonce,
      timestamp,
      signature,
    };
  };

  it("assigns a reviewer when requested and signed by the grant owner", async () => {
    const owner = Keypair.random();
    const reviewer = Keypair.random();
    const grant = await createGrant(owner.publicKey());

    const payload = signAddReviewerPayload(owner, grant.id, reviewer.publicKey());
    const response = await request(app).post("/grant_reviewers").send(payload);

    expect(response.status).toBe(201);
    expect(response.body.data).toMatchObject({
      grantId: grant.id,
      reviewerStellarAddress: reviewer.publicKey(),
    });

    const listResponse = await request(app).get(`/grant_reviewers/grant/${grant.id}`);
    expect(listResponse.status).toBe(200);
    expect(listResponse.body.data).toHaveLength(1);
    expect(listResponse.body.data[0].reviewerStellarAddress).toBe(reviewer.publicKey());
  });

  it("falls back to grant.recipient when grant.owner is empty", async () => {
    const recipient = Keypair.random();
    const reviewer = Keypair.random();
    const grant = await createGrant("", recipient.publicKey());

    const payload = signAddReviewerPayload(recipient, grant.id, reviewer.publicKey());
    const response = await request(app).post("/grant_reviewers").send(payload);

    expect(response.status).toBe(201);
    expect(response.body.data.reviewerStellarAddress).toBe(reviewer.publicKey());
  });

  it("returns 409 when assigning a duplicate reviewer to the same grant", async () => {
    const owner = Keypair.random();
    const reviewer = Keypair.random();
    const grant = await createGrant(owner.publicKey());

    const firstResponse = await request(app)
      .post("/grant_reviewers")
      .send(signAddReviewerPayload(owner, grant.id, reviewer.publicKey()));
    expect(firstResponse.status).toBe(201);

    const duplicateResponse = await request(app)
      .post("/grant_reviewers")
      .send(signAddReviewerPayload(owner, grant.id, reviewer.publicKey()));

    expect(duplicateResponse.status).toBe(409);
    expect(duplicateResponse.body).toEqual({
      error: "Reviewer already exists for this grant",
    });

    const persisted = await dataSource.getRepository(GrantReviewer).find({
      where: { grantId: grant.id, reviewerStellarAddress: reviewer.publicKey() },
    });
    expect(persisted).toHaveLength(1);
  });

  it("returns 409 when a signature nonce is replayed", async () => {
    const owner = Keypair.random();
    const reviewerOne = Keypair.random();
    const reviewerTwo = Keypair.random();
    const grant = await createGrant(owner.publicKey());
    const sharedNonce = randomUUID();

    const firstResponse = await request(app)
      .post("/grant_reviewers")
      .send(
        signAddReviewerPayload(owner, grant.id, reviewerOne.publicKey(), {
          nonce: sharedNonce,
        }),
      );
    expect(firstResponse.status).toBe(201);

    const replayResponse = await request(app)
      .post("/grant_reviewers")
      .send(
        signAddReviewerPayload(owner, grant.id, reviewerTwo.publicKey(), {
          nonce: sharedNonce,
        }),
      );

    expect(replayResponse.status).toBe(409);
    expect(replayResponse.body).toEqual({
      error: "Signature nonce has already been used",
    });
  });

  it("returns 403 when a non-owner tries to assign a reviewer", async () => {
    const owner = Keypair.random();
    const recipient = Keypair.random();
    const outsider = Keypair.random();
    const reviewer = Keypair.random();
    const grant = await createGrant(owner.publicKey(), recipient.publicKey());

    const outsiderResponse = await request(app)
      .post("/grant_reviewers")
      .send(signAddReviewerPayload(outsider, grant.id, reviewer.publicKey()));

    expect(outsiderResponse.status).toBe(403);
    expect(outsiderResponse.body).toEqual({
      error: "Only the grant owner can add reviewers",
    });

    const recipientResponse = await request(app)
      .post("/grant_reviewers")
      .send(signAddReviewerPayload(recipient, grant.id, reviewer.publicKey()));

    expect(recipientResponse.status).toBe(403);
    expect(recipientResponse.body).toEqual({
      error: "Only the grant owner can add reviewers",
    });

    const persisted = await dataSource.getRepository(GrantReviewer).find({
      where: { grantId: grant.id },
    });
    expect(persisted).toHaveLength(0);
  });

  it("returns 401 for an invalid or mismatched Stellar signature", async () => {
    const owner = Keypair.random();
    const attacker = Keypair.random();
    const reviewer = Keypair.random();
    const grant = await createGrant(owner.publicKey());

    // Signed by attacker while claiming to be owner
    const forgedPayload = signAddReviewerPayload(
      attacker,
      grant.id,
      reviewer.publicKey(),
      { address: owner.publicKey() },
    );
    const forgedResponse = await request(app)
      .post("/grant_reviewers")
      .send(forgedPayload);
    expect(forgedResponse.status).toBe(401);
    expect(forgedResponse.body).toEqual({ error: "Invalid Stellar signature" });

    // Malformed signature length
    const malformedPayload = signAddReviewerPayload(
      owner,
      grant.id,
      reviewer.publicKey(),
      { signature: Buffer.from("invalid-signature-bytes").toString("base64").padEnd(36, "A") },
    );
    const malformedResponse = await request(app)
      .post("/grant_reviewers")
      .send(malformedPayload);
    expect(malformedResponse.status).toBe(401);
    expect(malformedResponse.body).toEqual({ error: "Invalid Stellar signature" });
  });

  it("returns 400 for invalid payloads or expired timestamps", async () => {
    const owner = Keypair.random();
    const reviewer = Keypair.random();
    const grant = await createGrant(owner.publicKey());

    const invalidPayloadResponse = await request(app)
      .post("/grant_reviewers")
      .send({ grantId: -1, reviewerStellarAddress: "short" });
    expect(invalidPayloadResponse.status).toBe(400);
    expect(invalidPayloadResponse.body.error).toBe("Invalid payload");

    const expiredPayload = signAddReviewerPayload(
      owner,
      grant.id,
      reviewer.publicKey(),
      { timestamp: Date.now() - 6 * 60 * 1000 },
    );
    const expiredResponse = await request(app)
      .post("/grant_reviewers")
      .send(expiredPayload);
    expect(expiredResponse.status).toBe(400);
    expect(expiredResponse.body).toEqual({ error: "Expired intent timestamp" });
  });

  it("returns 404 when the target grant does not exist", async () => {
    const owner = Keypair.random();
    const reviewer = Keypair.random();
    const payload = signAddReviewerPayload(owner, 999999, reviewer.publicKey());

    const response = await request(app).post("/grant_reviewers").send(payload);

    expect(response.status).toBe(404);
    expect(response.body).toEqual({ error: "Grant not found" });
  });

  it("removes a reviewer and rejects invalid path parameters", async () => {
    const owner = Keypair.random();
    const reviewer = Keypair.random();
    const grant = await createGrant(owner.publicKey());

    const createResponse = await request(app)
      .post("/grant_reviewers")
      .send(signAddReviewerPayload(owner, grant.id, reviewer.publicKey()));
    expect(createResponse.status).toBe(201);
    const reviewerId = createResponse.body.data.id;

    const deleteResponse = await request(app).delete(`/grant_reviewers/${reviewerId}`);
    expect(deleteResponse.status).toBe(200);
    expect(deleteResponse.body).toEqual({ success: true });

    const listAfterDelete = await request(app).get(`/grant_reviewers/grant/${grant.id}`);
    expect(listAfterDelete.status).toBe(200);
    expect(listAfterDelete.body.data).toEqual([]);

    const invalidList = await request(app).get("/grant_reviewers/grant/not-a-number");
    expect(invalidList.status).toBe(400);

    const invalidDelete = await request(app).delete("/grant_reviewers/not-a-number");
    expect(invalidDelete.status).toBe(400);
  });
});
