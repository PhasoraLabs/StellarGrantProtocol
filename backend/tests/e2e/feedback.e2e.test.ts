import { Keypair } from "@stellar/stellar-sdk";
import request from "supertest";
import { beforeAll, afterAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { SignatureService } from "../../src/services/signature-service";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import { Activity } from "../../src/entities/Activity";
import { GrantReviewer } from "../../src/entities/GrantReviewer";
import { Grant } from "../../src/entities/Grant";

describe("Feedback E2E Tests", () => {
  let dataSource: DataSource;
  const sorobanClient = new MockSorobanContractClient();
  const signatureService = new SignatureService();

  const funderKey = Keypair.random();
  const reviewerKey = Keypair.random();
  const customRecipientKey = Keypair.random();

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  const setupData = async (app: any) => {
    // Seed grants from mock soroban client
    await request(app).get("/grants");

    // Retrieve and update grant 3 recipient
    const grantRepo = dataSource.getRepository(Grant);
    const grant3 = await grantRepo.findOne({ where: { id: 3 } });
    if (grant3) {
      grant3.recipient = customRecipientKey.publicKey();
      await grantRepo.save(grant3);
    }

    // Seed reviewer
    const reviewerRepo = dataSource.getRepository(GrantReviewer);
    await reviewerRepo.save({
      grantId: 3,
      reviewerStellarAddress: reviewerKey.publicKey(),
    });

    // Seed funder activity
    const activityRepo = dataSource.getRepository(Activity);
    await activityRepo.save({
      type: "grant_funded",
      entityType: "grant",
      entityId: 3,
      actorAddress: funderKey.publicKey(),
    });
  };

  it("handles feedback flows completely", async () => {
    const app = createApp(dataSource, sorobanClient);
    await setupData(app);

    // 1. GET feedback should initially return empty items
    const getRes1 = await request(app).get("/grants/3/feedback");
    expect(getRes1.status).toBe(200);
    expect(getRes1.body.data.averageRating).toBe(0);
    expect(getRes1.body.data.feedbackCount).toBe(0);
    expect(getRes1.body.data.items).toHaveLength(0);

    // 2. Reject feedback on non-completed grant (e.g. grant 1 is active)
    const payloadActiveGrant = {
      rating: 5,
      comment: "Should fail",
      role: "funder" as const,
      address: funderKey.publicKey(),
      nonce: "nonce-active",
      timestamp: Date.now(),
      signature: "dummy-sig",
    };
    const badGrantRes = await request(app)
      .post("/grants/1/feedback")
      .send(payloadActiveGrant);
    expect(badGrantRes.status).toBe(400);

    // 3. POST feedback - reject invalid signature
    const payloadInvalidSig = {
      rating: 5,
      comment: "Invalid signature",
      role: "reviewer" as const,
      address: reviewerKey.publicKey(),
      nonce: "nonce-invalid-sig",
      timestamp: Date.now(),
      signature: Buffer.from("x".repeat(64)).toString("base64"),
    };
    const invalidSigRes = await request(app)
      .post("/grants/3/feedback")
      .send(payloadInvalidSig);
    expect(invalidSigRes.status).toBe(401);

    // 4. POST feedback - reject unauthorized role (e.g. random user claiming reviewer)
    const randomKey = Keypair.random();
    const payloadUnauth = {
      rating: 4,
      comment: "I am not authorized",
      role: "reviewer" as const,
      address: randomKey.publicKey(),
      nonce: "nonce-unauth",
      timestamp: Date.now(),
    };
    const msgUnauth = signatureService.buildFeedbackIntentMessage({
      grantId: 3,
      ...payloadUnauth,
    });
    const sigUnauth = randomKey.sign(Buffer.from(msgUnauth, "utf8")).toString("base64");
    const unauthRes = await request(app)
      .post("/grants/3/feedback")
      .send({ ...payloadUnauth, signature: sigUnauth });
    expect(unauthRes.status).toBe(403);

    // 5. POST feedback - accept valid reviewer feedback
    const payloadReviewer = {
      rating: 4,
      comment: "Reviewer comments here.",
      role: "reviewer" as const,
      address: reviewerKey.publicKey(),
      nonce: "nonce-reviewer",
      timestamp: Date.now(),
    };
    const msgReviewer = signatureService.buildFeedbackIntentMessage({
      grantId: 3,
      ...payloadReviewer,
    });
    const sigReviewer = reviewerKey.sign(Buffer.from(msgReviewer, "utf8")).toString("base64");
    const reviewerRes = await request(app)
      .post("/grants/3/feedback")
      .send({ ...payloadReviewer, signature: sigReviewer });
    expect(reviewerRes.status).toBe(201);
    expect(reviewerRes.body.data.rating).toBe(4);

    // 6. POST feedback - accept valid funder feedback
    const payloadFunder = {
      rating: 5,
      comment: "Funder comments here.",
      role: "funder" as const,
      address: funderKey.publicKey(),
      nonce: "nonce-funder",
      timestamp: Date.now(),
    };
    const msgFunder = signatureService.buildFeedbackIntentMessage({
      grantId: 3,
      ...payloadFunder,
    });
    const sigFunder = funderKey.sign(Buffer.from(msgFunder, "utf8")).toString("base64");
    const funderRes = await request(app)
      .post("/grants/3/feedback")
      .send({ ...payloadFunder, signature: sigFunder });
    expect(funderRes.status).toBe(201);

    // 7. POST feedback - reject duplicate submission (funder again)
    const duplicateRes = await request(app)
      .post("/grants/3/feedback")
      .send({ ...payloadFunder, signature: sigFunder });
    expect(duplicateRes.status).toBe(409);
    expect(duplicateRes.body.rating).toBe(5);

    // 8. GET feedback should now return the average, count, and masked reviewerAddress
    const getRes2 = await request(app).get("/grants/3/feedback");
    expect(getRes2.status).toBe(200);
    expect(getRes2.body.data.averageRating).toBe(4.5);
    expect(getRes2.body.data.feedbackCount).toBe(2);
    expect(getRes2.body.data.items).toHaveLength(2);
    // privacy mask check
    expect(getRes2.body.data.items[0].reviewerAddress).toBeNull();
    expect(getRes2.body.data.items[1].reviewerAddress).toBeNull();
    // Verify sorting (descending by createdAt)
    expect(getRes2.body.data.items[0].role).toBe("funder");
    expect(getRes2.body.data.items[1].role).toBe("reviewer");
  });
});
