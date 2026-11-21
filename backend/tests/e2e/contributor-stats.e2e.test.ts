import { Keypair } from "@stellar/stellar-sdk";
import request from "supertest";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import { Contributor } from "../../src/entities/Contributor";
import { Grant } from "../../src/entities/Grant";
import { MilestoneProof } from "../../src/entities/MilestoneProof";
import { MilestoneApproval } from "../../src/entities/MilestoneApproval";

describe("GET /contributors/:address/stats", () => {
  let dataSource: DataSource;
  let app: ReturnType<typeof createApp>;

  const contributorAddress = Keypair.random().publicKey();
  const inactiveAddress = Keypair.random().publicKey();
  const otherAddress = Keypair.random().publicKey();
  const reviewerOne = Keypair.random().publicKey();
  const reviewerTwo = Keypair.random().publicKey();

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();

    const contributorRepo = dataSource.getRepository(Contributor);
    await contributorRepo.save([
      contributorRepo.create({
        address: contributorAddress,
        reputation: 250,
        totalGrantsCompleted: 2,
      }),
      contributorRepo.create({
        address: inactiveAddress,
        reputation: 0,
        totalGrantsCompleted: 0,
      }),
    ]);

    const grantRepo = dataSource.getRepository(Grant);
    await grantRepo.save([
      grantRepo.create({
        id: 501,
        title: "Completed grant",
        status: "completed",
        owner: otherAddress,
        recipient: contributorAddress,
        totalAmount: "100",
        isDraft: false,
      }),
      grantRepo.create({
        id: 502,
        title: "Active grant",
        status: "active",
        owner: otherAddress,
        recipient: contributorAddress,
        totalAmount: "50",
        isDraft: false,
      }),
      grantRepo.create({
        id: 503,
        title: "Draft grant",
        status: "completed",
        owner: otherAddress,
        recipient: contributorAddress,
        totalAmount: "25.5",
        isDraft: true,
      }),
      grantRepo.create({
        id: 504,
        title: "Someone else's grant",
        status: "completed",
        owner: otherAddress,
        recipient: otherAddress,
        totalAmount: "999",
        isDraft: false,
      }),
    ]);

    const proofRepo = dataSource.getRepository(MilestoneProof);
    await proofRepo.save([
      proofRepo.create({
        grantId: 501,
        milestoneIdx: 0,
        proofCid: "bafy-proof-501-0",
        submittedBy: contributorAddress,
        signature: "signature-501-0",
        nonce: "nonce-501-0",
      }),
      proofRepo.create({
        grantId: 501,
        milestoneIdx: 1,
        proofCid: "bafy-proof-501-1",
        submittedBy: contributorAddress,
        signature: "signature-501-1",
        nonce: "nonce-501-1",
      }),
      proofRepo.create({
        grantId: 502,
        milestoneIdx: 0,
        proofCid: "bafy-proof-502-0",
        submittedBy: contributorAddress,
        signature: "signature-502-0",
        nonce: "nonce-502-0",
      }),
      // Draft grants are excluded from the contributor's stats.
      proofRepo.create({
        grantId: 503,
        milestoneIdx: 0,
        proofCid: "bafy-proof-503-0",
        submittedBy: contributorAddress,
        signature: "signature-503-0",
        nonce: "nonce-503-0",
      }),
    ]);

    const approvalRepo = dataSource.getRepository(MilestoneApproval);
    await approvalRepo.save([
      // Two reviewers approving the same milestone count only once.
      approvalRepo.create({
        grantId: 501,
        milestoneIdx: 0,
        reviewerStellarAddress: reviewerOne,
        approved: true,
      }),
      approvalRepo.create({
        grantId: 501,
        milestoneIdx: 0,
        reviewerStellarAddress: reviewerTwo,
        approved: true,
      }),
      approvalRepo.create({
        grantId: 501,
        milestoneIdx: 1,
        reviewerStellarAddress: reviewerOne,
        approved: false,
      }),
      approvalRepo.create({
        grantId: 502,
        milestoneIdx: 0,
        reviewerStellarAddress: reviewerOne,
        approved: true,
      }),
    ]);

    app = createApp(dataSource, new MockSorobanContractClient());
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  it("summarises milestones and earnings for a contributor", async () => {
    const response = await request(app).get(
      `/contributors/${contributorAddress}/stats`,
    );

    expect(response.status).toBe(200);
    expect(response.body.data).toEqual({
      address: contributorAddress,
      reputationScore: 250,
      grantsCompleted: 2,
      totalGrants: 2,
      completedGrants: 1,
      activeGrants: 1,
      milestonesSubmitted: 3,
      milestonesApproved: 2,
      approvalRate: 2 / 3,
      totalEarned: "100",
    });
  });

  it("returns zeroed stats for a contributor without grants", async () => {
    const response = await request(app).get(
      `/contributors/${inactiveAddress}/stats`,
    );

    expect(response.status).toBe(200);
    expect(response.body.data).toEqual({
      address: inactiveAddress,
      reputationScore: 0,
      grantsCompleted: 0,
      totalGrants: 0,
      completedGrants: 0,
      activeGrants: 0,
      milestonesSubmitted: 0,
      milestonesApproved: 0,
      approvalRate: 0,
      totalEarned: "0",
    });
  });

  it("returns 404 for an unknown contributor", async () => {
    const response = await request(app).get(
      `/contributors/${Keypair.random().publicKey()}/stats`,
    );

    expect(response.status).toBe(404);
    expect(response.body.error).toBe("Contributor not found");
  });

  it("rejects an invalid Stellar address", async () => {
    const response = await request(app).get("/contributors/not-an-address/stats");

    expect(response.status).toBe(400);
  });
});
