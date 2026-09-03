import request from "supertest";
import { beforeAll, afterAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import { Grant } from "../../src/entities/Grant";
import { Contributor } from "../../src/entities/Contributor";
import { MilestoneProof } from "../../src/entities/MilestoneProof";

describe("Search API e2e", () => {
  let dataSource: DataSource;
  const sorobanClient = new MockSorobanContractClient();

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();

    // Seed some data for searching
    const grantRepo = dataSource.getRepository(Grant);
    const contributorRepo = dataSource.getRepository(Contributor);
    const proofRepo = dataSource.getRepository(MilestoneProof);

    await grantRepo.save([
      {
        id: 101,
        title: "Stellar Wallet Integration",
        status: "active",
        recipient: "GBRPYHIL2C2WBO36G6UIGR2PA4M3TQ7VOY3RTMAL4LRRA67ZOHQ65SZD",
        totalAmount: "1000000",
        tags: "wallet,stellar,integration",
      },
      {
        id: 102,
        title: "DEX Aggregator",
        status: "pending",
        recipient: "GCBQ6JQXQTVV7T7OUVPR4Q6PGACCUAKS6S2YDG3YQYQYRR2NJB5A6NAA",
        totalAmount: "500000",
        tags: "dex,defi,aggregator",
      }
    ]);

    await contributorRepo.save([
      {
        address: "GDZAPKZFP3PVPRMDG6WQVIMZLQ5J3FZGQ27BFLDL3YQSM6L7LS6AXEX",
        reputation: 500,
        email: "alice@example.com",
      }
    ]);

    await proofRepo.save([
      {
        grantId: 101,
        milestoneIdx: 0,
        proofCid: "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco",
        description: "Initial research and design document for wallet sync.",
        submittedBy: "GBRPYHIL2C2WBO36G6UIGR2PA4M3TQ7VOY3RTMAL4LRRA67ZOHQ65SZD",
        signature: "sig123",
        nonce: "nonce123",
      }
    ]);
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  it("returns results for grant title search", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/search?q=Integration");

    expect(response.status).toBe(200);
    expect(response.body.data.length).toBeGreaterThanOrEqual(1);
    const match = response.body.data.find((r: any) => String(r.id) === "101");
    expect(match.name).toBe("Stellar Wallet Integration");
    expect(match.type).toBe("grant");
  });

  it("returns results for contributor email search", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/search?q=alice");

    expect(response.status).toBe(200);
    expect(response.body.data).toHaveLength(1);
    expect(response.body.data[0].type).toBe("contributor");
    expect(response.body.data[0].id).toBe("GDZAPKZFP3PVPRMDG6WQVIMZLQ5J3FZGQ27BFLDL3YQSM6L7LS6AXEX");
  });

  it("returns results for milestone description search", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/search?q=research");

    expect(response.status).toBe(200);
    expect(response.body.data).toHaveLength(1);
    expect(response.body.data[0].type).toBe("milestone");
    expect(response.body.data[0].name).toContain("Initial research");
  });

  it("returns empty results for non-matching query", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/search?q=nonexistent");

    expect(response.status).toBe(200);
    expect(response.body.data).toHaveLength(0);
  });

  it("returns empty results for short query", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/search?q=a");

    expect(response.status).toBe(400);
  });
});
