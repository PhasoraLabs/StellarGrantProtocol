import request from "supertest";
import { beforeAll, afterAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";

describe("Leaderboard API e2e", () => {
  let dataSource: DataSource;
  const sorobanClient = new MockSorobanContractClient();

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  it("returns an empty leaderboard initially", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/leaderboard");

    expect(response.status).toBe(200);
    expect(response.body.data).toHaveLength(0);
    expect(response.body.meta.total).toBe(0);
  });

  it("populates the leaderboard after syncing grants", async () => {
    const app = createApp(dataSource, sorobanClient);
    
    // Trigger sync by calling /grants
    await request(app).get("/grants");

    const response = await request(app).get("/leaderboard");

    expect(response.status).toBe(200);
    // There are 4 unique recipients in mockGrants
    expect(response.body.data).toHaveLength(4);
    expect(response.body.meta.total).toBe(4);
    
    // Check if rankings are present
    expect(response.body.data[0].reputation).toBe(100);
  });

  it("supports monthly filtering", async () => {
    const app = createApp(dataSource, sorobanClient);
    
    // Since we just synced, all 100 rep gains are within the last 30 days
    const response = await request(app).get("/leaderboard?period=monthly");

    expect(response.status).toBe(200);
    expect(response.body.data).toHaveLength(4);
    expect(response.body.data[0].reputation).toBe(100);
  });

  it("respects limit and page", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/leaderboard?limit=2&page=1");

    expect(response.status).toBe(200);
    expect(response.body.data).toHaveLength(2);
    expect(response.body.meta.total).toBe(4);
    expect(response.body.meta.totalPages).toBe(2);
  });
});
