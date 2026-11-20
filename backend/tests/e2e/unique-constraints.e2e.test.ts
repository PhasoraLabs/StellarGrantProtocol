import express from "express";
import request from "supertest";
import { Keypair } from "@stellar/stellar-sdk";
import { getMetadataArgsStorage } from "typeorm";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { buildDataSource, shouldSynchronize } from "../../src/db/data-source";
import { Dispute } from "../../src/entities/Dispute";
import { Grant } from "../../src/entities/Grant";
import { GrantReviewer } from "../../src/entities/GrantReviewer";
import { MilestoneApproval } from "../../src/entities/MilestoneApproval";
import { UsedNonce } from "../../src/entities/UsedNonce";
import { buildMilestoneApprovalRouter } from "../../src/routes/milestone-approvals";
import { NonceService } from "../../src/services/nonce-service";
import { SignatureService } from "../../src/services/signature-service";

const uniquesFor = (target: Function) =>
  getMetadataArgsStorage()
    .uniques.filter((u) => u.target === target)
    .map((u) => u.columns);

describe("schema guards", () => {
  it("never synchronizes the schema in production", () => {
    expect(shouldSynchronize("production")).toBe(false);
    expect(shouldSynchronize("development")).toBe(true);
    expect(shouldSynchronize("test")).toBe(true);
  });

  it("declares one dispute per (grantId, milestoneIdx)", () => {
    expect(uniquesFor(Dispute)).toContainEqual(["grantId", "milestoneIdx"]);
  });

  it("declares one approval per reviewer per milestone", () => {
    expect(uniquesFor(MilestoneApproval)).toContainEqual([
      "grantId",
      "milestoneIdx",
      "reviewerStellarAddress",
    ]);
  });
});

describe("milestone approval resubmission", () => {
  let dataSource: DataSource;
  let app: express.Express;
  let reviewer: Keypair;
  const signatureService = new SignatureService();

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();

    reviewer = Keypair.random();
    await dataSource.getRepository(Grant).save({
      id: 1,
      title: "Test grant",
      status: "active",
      owner: "GOWNER",
      recipient: "GRECIPIENT",
      totalAmount: "1000",
    });
    await dataSource.getRepository(GrantReviewer).save({
      grantId: 1,
      reviewerStellarAddress: reviewer.publicKey(),
    });

    app = express();
    app.use(express.json());
    app.use(
      "/milestone-approvals",
      buildMilestoneApprovalRouter(
        dataSource.getRepository(MilestoneApproval),
        dataSource.getRepository(Grant),
        dataSource.getRepository(GrantReviewer),
        signatureService,
        new NonceService(dataSource.getRepository(UsedNonce)),
      ),
    );
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) await dataSource.destroy();
  });

  it("updates the existing vote instead of inserting a duplicate", async () => {
    const repo = dataSource.getRepository(MilestoneApproval);
    const submitVote = async (approved: boolean, nonce: string) => {
      const intent = {
        grantId: 1,
        milestoneIdx: 0,
        reviewerStellarAddress: reviewer.publicKey(),
        approved,
        address: reviewer.publicKey(),
        nonce,
        timestamp: Date.now(),
      };
      const signature = reviewer
        .sign(Buffer.from(signatureService.buildMilestoneApprovalIntentMessage(intent)))
        .toString("base64");
      return request(app).post("/milestone-approvals").send({ ...intent, signature });
    };

    const first = await submitVote(true, "first-nonce");
    const second = await submitVote(false, "second-nonce");

    expect(first.status).toBe(201);
    expect(second.status).toBe(200);
    const rows = await repo.find({ where: { grantId: 1, milestoneIdx: 0 } });
    expect(rows).toHaveLength(1);
    expect(rows[0].approved).toBe(false);
  });

  it("rejects a duplicate approval row at the database level", async () => {
    const repo = dataSource.getRepository(MilestoneApproval);
    const row = { grantId: 2, milestoneIdx: 1, reviewerStellarAddress: "GDUP", approved: true };
    await repo.save(repo.create(row));
    await expect(repo.save(repo.create(row))).rejects.toThrow();
  });
});
