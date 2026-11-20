import { randomUUID } from "node:crypto";
import { Keypair } from "@stellar/stellar-sdk";
import express from "express";
import request from "supertest";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { DataSource } from "typeorm";
import { buildDataSource } from "../../src/db/data-source";
import { buildMilestoneAppealsRouter } from "../../src/routes/milestone-appeals";
import { Grant } from "../../src/entities/Grant";
import { GrantReviewer } from "../../src/entities/GrantReviewer";
import { MilestoneApproval } from "../../src/entities/MilestoneApproval";
import { MilestoneProof } from "../../src/entities/MilestoneProof";
import { Activity } from "../../src/entities/Activity";
import { WebhookDispatcher } from "../../src/services/webhook-dispatcher";
import { NotificationService } from "../../src/services/notification-service";

/** Mirrors the signed intent messages built by routes/milestone-appeals.ts. */
const appealIntent = (
  grantId: number,
  milestoneIdx: number,
  reason: string,
  address: string,
  nonce: string,
  timestamp: number,
) =>
  ["stellargrant:appeal:v1", grantId, milestoneIdx, reason, address, nonce, timestamp].join(
    "|",
  );

const voteIntent = (
  grantId: number,
  milestoneIdx: number,
  reviewer: string,
  uphold: boolean,
  nonce: string,
  timestamp: number,
) =>
  ["stellargrant:appeal_vote:v1", grantId, milestoneIdx, reviewer, uphold.toString(), nonce, timestamp].join(
    "|",
  );

describe("milestone appeal lifecycle", () => {
  let dataSource: DataSource;
  let app: express.Express;
  const webhookDispatcher = {
    dispatch: vi.fn().mockResolvedValue(undefined),
  } as unknown as WebhookDispatcher;
  const notificationService = {
    notifyUser: vi.fn(),
  } as unknown as NotificationService;

  let nextGrantId = 600;

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();

    app = express();
    app.use(express.json());
    app.use(
      "/",
      buildMilestoneAppealsRouter(dataSource, webhookDispatcher, notificationService),
    );
  });

  beforeEach(() => {
    vi.mocked(webhookDispatcher.dispatch).mockClear();
    vi.mocked(notificationService.notifyUser).mockClear();
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  const seedScenario = async (options: {
    recipient: Keypair;
    reviewers: Keypair[];
    withRejection?: boolean;
    withProof?: boolean;
  }) => {
    const grantId = nextGrantId++;
    const milestoneIdx = 0;
    const recipientAddress = options.recipient.publicKey();

    const grantRepo = dataSource.getRepository(Grant);
    await grantRepo.save(
      grantRepo.create({
        id: grantId,
        title: `Appeal test grant ${grantId}`,
        status: "active",
        owner: recipientAddress,
        recipient: recipientAddress,
        totalAmount: "100",
        isDraft: false,
      }),
    );

    const reviewerRepo = dataSource.getRepository(GrantReviewer);
    await reviewerRepo.save(
      options.reviewers.map((reviewer) =>
        reviewerRepo.create({
          grantId,
          reviewerStellarAddress: reviewer.publicKey(),
        }),
      ),
    );

    if (options.withRejection !== false) {
      const approvalRepo = dataSource.getRepository(MilestoneApproval);
      await approvalRepo.save(
        approvalRepo.create({
          grantId,
          milestoneIdx,
          reviewerStellarAddress: options.reviewers[0].publicKey(),
          approved: false,
        }),
      );
    }

    if (options.withProof !== false) {
      const proofRepo = dataSource.getRepository(MilestoneProof);
      await proofRepo.save(
        proofRepo.create({
          grantId,
          milestoneIdx,
          proofCid: `bafy-appeal-proof-${grantId}`,
          submittedBy: recipientAddress,
          signature: "test-proof-signature",
          nonce: `proof-nonce-${grantId}`,
        }),
      );
    }

    return { grantId, milestoneIdx };
  };

  const openAppeal = (
    grantId: number,
    milestoneIdx: number,
    keypair: Keypair,
    options: {
      reason?: string;
      nonce?: string;
      timestamp?: number;
      signature?: string;
    } = {},
  ) => {
    const reason = options.reason ?? "Milestone was rejected by mistake";
    const address = keypair.publicKey();
    const nonce = options.nonce ?? randomUUID();
    const timestamp = options.timestamp ?? Date.now();
    const signature =
      options.signature ??
      keypair
        .sign(
          Buffer.from(
            appealIntent(grantId, milestoneIdx, reason, address, nonce, timestamp),
            "utf8",
          ),
        )
        .toString("base64");

    return request(app)
      .post(`/grants/${grantId}/milestones/${milestoneIdx}/appeal`)
      .send({ address, reason, signature, nonce, timestamp });
  };

  const voteOnAppeal = (
    grantId: number,
    milestoneIdx: number,
    keypair: Keypair,
    uphold: boolean,
    options: { nonce?: string; timestamp?: number; signature?: string } = {},
  ) => {
    const reviewer = keypair.publicKey();
    const nonce = options.nonce ?? randomUUID();
    const timestamp = options.timestamp ?? Date.now();
    const signature =
      options.signature ??
      keypair
        .sign(
          Buffer.from(
            voteIntent(grantId, milestoneIdx, reviewer, uphold, nonce, timestamp),
            "utf8",
          ),
        )
        .toString("base64");

    return request(app)
      .post(`/grants/${grantId}/milestones/${milestoneIdx}/appeal/vote`)
      .send({ address: reviewer, uphold, signature, nonce, timestamp });
  };

  it("opens an appeal as pending and notifies the grant reviewers", async () => {
    const recipient = Keypair.random();
    const reviewers = [Keypair.random(), Keypair.random()];
    const { grantId, milestoneIdx } = await seedScenario({ recipient, reviewers });

    const response = await openAppeal(grantId, milestoneIdx, recipient);

    expect(response.status).toBe(201);
    expect(response.body.data.status).toBe("pending");
    expect(response.body.data.reviewerVotes).toEqual([]);

    const status = await request(app).get(
      `/grants/${grantId}/milestones/${milestoneIdx}/appeal`,
    );
    expect(status.status).toBe(200);
    expect(status.body.data).toMatchObject({
      status: "pending",
      votesFor: 0,
      votesAgainst: 0,
      votesRemaining: 2,
    });

    expect(notificationService.notifyUser).toHaveBeenCalledTimes(2);
    expect(webhookDispatcher.dispatch).toHaveBeenCalledWith(
      "milestone.appealed",
      expect.objectContaining({ grantId, milestoneIdx }),
    );
  });

  it("does not open an appeal when the milestone was not rejected", async () => {
    const recipient = Keypair.random();
    const reviewers = [Keypair.random()];
    const { grantId, milestoneIdx } = await seedScenario({
      recipient,
      reviewers,
      withRejection: false,
    });

    const response = await openAppeal(grantId, milestoneIdx, recipient);

    expect(response.status).toBe(400);
    expect(response.body.error).toBe("Milestone must be rejected to open an appeal");
  });

  it("only lets the grant recipient open an appeal", async () => {
    const recipient = Keypair.random();
    const reviewers = [Keypair.random()];
    const { grantId, milestoneIdx } = await seedScenario({ recipient, reviewers });

    const response = await openAppeal(grantId, milestoneIdx, Keypair.random());

    expect(response.status).toBe(403);
  });

  it("rejects a second appeal for the same milestone", async () => {
    const recipient = Keypair.random();
    const reviewers = [Keypair.random()];
    const { grantId, milestoneIdx } = await seedScenario({ recipient, reviewers });

    expect((await openAppeal(grantId, milestoneIdx, recipient)).status).toBe(201);
    const duplicate = await openAppeal(grantId, milestoneIdx, recipient);

    expect(duplicate.status).toBe(409);
  });

  it("keeps the appeal pending and counts votes until every reviewer has voted", async () => {
    const recipient = Keypair.random();
    const reviewers = [Keypair.random(), Keypair.random()];
    const { grantId, milestoneIdx } = await seedScenario({ recipient, reviewers });

    expect((await openAppeal(grantId, milestoneIdx, recipient)).status).toBe(201);

    const firstVote = await voteOnAppeal(grantId, milestoneIdx, reviewers[0], true);
    expect(firstVote.status).toBe(200);
    expect(firstVote.body.data.status).toBe("pending");

    const status = await request(app).get(
      `/grants/${grantId}/milestones/${milestoneIdx}/appeal`,
    );
    expect(status.body.data).toMatchObject({
      status: "pending",
      votesFor: 1,
      votesAgainst: 0,
      votesRemaining: 1,
    });

    expect((await voteOnAppeal(grantId, milestoneIdx, reviewers[0], true)).status).toBe(
      409,
    );
    expect((await voteOnAppeal(grantId, milestoneIdx, Keypair.random(), true)).status).toBe(
      403,
    );
    expect(
      (
        await voteOnAppeal(grantId, milestoneIdx, reviewers[1], true, {
          timestamp: Date.now() - 10 * 60 * 1000,
        })
      ).status,
    ).toBe(400);
    expect(
      (
        await voteOnAppeal(grantId, milestoneIdx, reviewers[1], true, {
          signature: Buffer.from("x".repeat(64)).toString("base64"),
        })
      ).status,
    ).toBe(401);
  });

  it("resolves to upheld when a majority of reviewers uphold the appeal", async () => {
    const recipient = Keypair.random();
    const reviewers = [Keypair.random(), Keypair.random(), Keypair.random()];
    const { grantId, milestoneIdx } = await seedScenario({ recipient, reviewers });

    expect((await openAppeal(grantId, milestoneIdx, recipient)).status).toBe(201);
    expect((await voteOnAppeal(grantId, milestoneIdx, reviewers[0], true)).status).toBe(
      200,
    );
    expect((await voteOnAppeal(grantId, milestoneIdx, reviewers[1], true)).body.data.status).toBe(
      "pending",
    );

    const finalVote = await voteOnAppeal(grantId, milestoneIdx, reviewers[2], false);
    expect(finalVote.status).toBe(200);
    expect(finalVote.body.data.status).toBe("upheld");
    expect(finalVote.body.data.resolvedAt).toBeTruthy();

    expect(webhookDispatcher.dispatch).toHaveBeenCalledWith(
      "milestone.appeal_resolved",
      expect.objectContaining({ grantId, milestoneIdx, status: "upheld" }),
    );

    // The rejection stays in place when the appeal is upheld.
    const approvals = await dataSource.getRepository(MilestoneApproval).find({
      where: { grantId, milestoneIdx },
    });
    expect(approvals.some((approval) => approval.approved === false)).toBe(true);

    const lateVote = await voteOnAppeal(grantId, milestoneIdx, reviewers[0], true);
    expect(lateVote.status).toBe(400);
    expect(lateVote.body.error).toBe("Appeal is already resolved");
  });

  it("resolves to denied and reactivates the milestone when the appeal is rejected", async () => {
    const recipient = Keypair.random();
    const reviewers = [Keypair.random(), Keypair.random()];
    const { grantId, milestoneIdx } = await seedScenario({ recipient, reviewers });

    expect((await openAppeal(grantId, milestoneIdx, recipient)).status).toBe(201);
    expect((await voteOnAppeal(grantId, milestoneIdx, reviewers[0], false)).status).toBe(
      200,
    );

    const finalVote = await voteOnAppeal(grantId, milestoneIdx, reviewers[1], false);
    expect(finalVote.status).toBe(200);
    expect(finalVote.body.data.status).toBe("denied");

    const approvals = await dataSource.getRepository(MilestoneApproval).find({
      where: { grantId, milestoneIdx },
    });
    expect(approvals).toHaveLength(0);

    const reactivation = await dataSource.getRepository(Activity).findOne({
      where: { type: "milestone_reactivated", entityType: "milestone_proof" },
    });
    expect(reactivation).toBeDefined();
    expect(reactivation?.data).toMatchObject({ grantId, milestoneIdx });

    expect(webhookDispatcher.dispatch).toHaveBeenCalledWith(
      "milestone.appeal_resolved",
      expect.objectContaining({ status: "denied" }),
    );
  });

  it("returns 404 when voting on an appeal that does not exist", async () => {
    const reviewer = Keypair.random();
    const response = await voteOnAppeal(9999, 0, reviewer, true);

    expect(response.status).toBe(404);
  });
});
