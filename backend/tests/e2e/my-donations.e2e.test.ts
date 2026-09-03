import request from "supertest";
import { describe, it, beforeAll, afterAll, expect } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { DataSourceOptions } from "typeorm";
import { FeeCollection } from "../../src/entities/FeeCollection";
import { Grant } from "../../src/entities/Grant";
import { MilestoneProof } from "../../src/entities/MilestoneProof";
import { GrantReviewer } from "../../src/entities/GrantReviewer";
import { Milestone } from "../../src/entities/Milestone";
import { Contributor } from "../../src/entities/Contributor";
import { User } from "../../src/entities/User";
import { MilestoneApproval } from "../../src/entities/MilestoneApproval";
import { Community } from "../../src/entities/Community";
import { Role } from "../../src/entities/Role";
import { UserRole } from "../../src/entities/UserRole";

const TEST_ADDRESS = "GTESTFUNDERTOKEN123";
const OTHER_ADDRESS = "GOTHERFUNDERTOKEN456";

// Helper to insert test data
async function seedDonations(ds: DataSource) {
  const grantRepo = ds.getRepository(Grant);
  const feeRepo = ds.getRepository(FeeCollection);
  const grant = await grantRepo.save({
    id: 1,
    title: "Test Grant",
    status: "active",
    recipient: "GRECIPIENT1",
    totalAmount: "1000",
    tags: null,
    localizedMetadata: null,
    updatedAt: new Date(),
    proofs: [],
    reviewers: [],
  });
  await feeRepo.save([
    {
      grantId: grant.id,
      funderAddress: TEST_ADDRESS,
      token: "USDC",
      totalContribution: "100",
      feeAmount: "2",
      feePercentage: "2",
      createdAt: new Date(),
    },
    {
      grantId: grant.id,
      funderAddress: TEST_ADDRESS,
      token: "XLM",
      totalContribution: "50",
      feeAmount: "1",
      feePercentage: "2",
      createdAt: new Date(),
    },
    {
      grantId: grant.id,
      funderAddress: OTHER_ADDRESS,
      token: "USDC",
      totalContribution: "200",
      feeAmount: "4",
      feePercentage: "2",
      createdAt: new Date(),
    },
  ]);
}


describe("GET /my-donations", () => {
  let dataSource: DataSource;
  let app: ReturnType<typeof createApp>;

  beforeAll(async () => {
    // Custom DataSource with FeeCollection included
    const options: DataSourceOptions = {
      type: "sqljs",
      location: "memory",
      autoSave: false,
      entities: [Grant, Community, Milestone, FeeCollection, MilestoneProof, GrantReviewer, Contributor, User, MilestoneApproval, Role, UserRole],
      synchronize: true,
    };
    dataSource = new DataSource(options);
    await dataSource.initialize();
    await seedDonations(dataSource);
    app = createApp(dataSource, {} as any);
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  it("returns all donations for the authenticated funder, grouped by token", async () => {
    const res = await request(app)
      .get("/my-donations")
      .set("x-stellar-address", TEST_ADDRESS)
      .expect(200);
    expect(res.body.donations.length).toBe(2);
    expect(res.body.totals.USDC).toBe("100");
    expect(res.body.totals.XLM).toBe("50");
  });

  it("filters donations by token type", async () => {
    const res = await request(app)
      .get("/my-donations?token=USDC")
      .set("x-stellar-address", TEST_ADDRESS)
      .expect(200);
    expect(res.body.donations.length).toBe(1);
    expect(res.body.donations[0].token).toBe("USDC");
    expect(res.body.totals.USDC).toBe("100");
    expect(res.body.totals.XLM).toBeUndefined();
  });

  it("returns 401 if not authenticated", async () => {
    const res = await request(app).get("/my-donations").expect(401);
    expect(res.body.error).toBe(true);
    expect(res.body.code).toBe("UNAUTHORIZED");
  });
});
