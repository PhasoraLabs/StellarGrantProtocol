import request from "supertest";
import { describe, it, beforeAll, beforeEach, afterAll, expect, vi } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import * as emailService from "../../src/services/email-service";
import * as signatureServiceModule from "../../src/services/signature-service";
import { Grant } from "../../src/entities/Grant";

// Mock sendEmail to avoid sending real emails
vi.spyOn(emailService, "sendEmail").mockImplementation(async () => {});
// Mock signatureService.verify to always return true
vi.spyOn(signatureServiceModule.SignatureService.prototype, "verify").mockReturnValue(true);

const owner = {
  email: "owner@example.com",
  stellarAddress: "GBRPYHIL2C2WBO36G6UIGR2PA4M3TQ7VOY3RTMAL4LRRA67ZOHQ65SZD",
  notifyMilestoneSubmitted: true,
  notifyMilestoneApproved: true,
};
const reviewer = {
  email: "reviewer@example.com",
  stellarAddress: "GCBQ6JQXQTVV7T7OUVPR4Q6PGACCUAKS6S2YDG3YQYQYRR2NJB5A6NAA",
  notifyMilestoneSubmitted: true,
  notifyMilestoneApproved: false,
};

describe("Email notification and user preference integration", () => {
  let dataSource: DataSource;
  let app: ReturnType<typeof createApp>;
  const sorobanClient = new MockSorobanContractClient();

  // Single beforeAll — no duplicate
  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();

    // Insert a grant for testing (id: 1, matches owner.stellarAddress)
    const grantRepo = dataSource.getRepository(Grant);
    await grantRepo.save({
      id: 1,
      title: "Test Grant",
      status: "active",
      recipient: owner.stellarAddress,
      totalAmount: "1000",
      tags: null,
      updatedAt: new Date(),
      proofs: [],
      reviewers: [],
    });

    // Create the app once and reuse across all tests so registered
    // users and reviewer associations persist between tests.
    app = createApp(dataSource, sorobanClient);

    // Register users and wire up the reviewer once for the whole suite
    await request(app).post("/users/register").send(owner);
    await request(app).post("/users/register").send(reviewer);
    await request(app)
      .post("/grant_reviewers")
      .send({ grantId: 1, reviewerStellarAddress: reviewer.stellarAddress });
  });

  // Clear the mock call log between tests so each test only sees its own calls
  beforeEach(() => {
    (emailService.sendEmail as any).mockClear();
  });

  it("sends emails to grant owner and reviewers on milestone submission if opted-in", async () => {
    // Verify reviewer relation is loaded correctly
    const grantRepo = dataSource.getRepository(Grant);
    const grantWithReviewers = await grantRepo.findOne({
      where: { id: 1 },
      relations: ["reviewers"],
    });
    expect(grantWithReviewers).toBeDefined();
    expect(grantWithReviewers!.reviewers.length).toBeGreaterThan(0);

    // Submit milestone
    const payload = {
      grantId: 1,
      milestoneIdx: 0,
      proofCid: "bafybeihk-test-proof",
      submittedBy: reviewer.stellarAddress,
      signature: "a".repeat(32), // min 32 chars required by schema
      nonce: "nonce-test",
      timestamp: Date.now(),
    };
    await request(app).post("/milestone_proof").send(payload);

    // Both owner and reviewer should have been emailed
    expect(emailService.sendEmail).toHaveBeenCalled();
    const calls = (emailService.sendEmail as any).mock.calls;
    const recipients = calls.map((c: any[]) => c[0].to);
    expect(recipients).toContain(owner.email);
    expect(recipients).toContain(reviewer.email);
  });

  it("does not send email if user opts out", async () => {
    // Update reviewer to opt out of milestone submission notifications
    await request(app)
      .post("/users/register")
      .send({ ...reviewer, notifyMilestoneSubmitted: false });

    // Submit a new milestone (different index to avoid unique-constraint clash)
    const payload = {
      grantId: 1,
      milestoneIdx: 1,
      proofCid: "bafybeihk-test-proof2",
      submittedBy: reviewer.stellarAddress,
      signature: "a".repeat(32), // min 32 chars required by schema
      nonce: "nonce-test2",
      timestamp: Date.now(),
    };
    await request(app).post("/milestone_proof").send(payload);

    // Reviewer opted out — should receive no email in this test
    const calls = (emailService.sendEmail as any).mock.calls;
    const reviewerEmails = calls.filter((c: any[]) => c[0].to === reviewer.email);
    expect(reviewerEmails.length).toBe(0);
  });

  it("sends email to owner on milestone approval if opted-in", async () => {
    // Approve milestone
    await request(app).post("/milestone_approvals_notify/notify").send({
      grantId: 1,
      milestoneIdx: 0,
      reviewerStellarAddress: reviewer.stellarAddress,
      approved: true,
    });

    // Owner has notifyMilestoneApproved: true — should receive email
    const calls = (emailService.sendEmail as any).mock.calls;
    const ownerEmails = calls.filter((c: any[]) => c[0].to === owner.email);
    expect(ownerEmails.length).toBeGreaterThan(0);
  });
});