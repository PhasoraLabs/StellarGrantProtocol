import { beforeAll, afterAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { TestDbSetup } from "../setup/test-db-setup";
import { GrantSyncService } from "../../src/services/grant-sync-service";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import { Grant } from "../../src/entities/Grant";
import { Contributor } from "../../src/entities/Contributor";

describe("Grant Flow Integration (Testcontainers)", () => {
  let testDb: TestDbSetup;
  let dataSource: DataSource;
  const sorobanClient = new MockSorobanContractClient();

  beforeAll(async () => {
    testDb = new TestDbSetup();
    dataSource = await testDb.start();
  }, 120000); // Higher timeout for container startup

  afterAll(async () => {
    await testDb.stop();
  });

  it("syncs all grants from Soroban and persists to PostgreSQL", async () => {
    const syncService = new GrantSyncService(dataSource, sorobanClient);

    await syncService.syncAllGrants();

    const grantRepo = dataSource.getRepository(Grant);
    const count = await grantRepo.count();

    // mockGrants has 4 items
    expect(count).toBe(4);

    const firstGrant = await grantRepo.findOne({ where: { id: 1 } });
    expect(firstGrant).toBeDefined();
    expect(firstGrant?.title).toBe("Open Source Grants Q2");
  });

  it("syncs contributor scores and updates reputation correctly", async () => {
    const syncService = new GrantSyncService(dataSource, sorobanClient);
    const recipient = "GDATAPRIVACYRECPNT0000000000000000000000000000000000000"; // Grant #3 recipient (completed)

    await syncService.syncGrant(3);

    const contributorRepo = dataSource.getRepository(Contributor);
    const contributor = await contributorRepo.findOne({ where: { address: recipient } });

    expect(contributor).toBeDefined();
    expect(contributor?.reputation).toBe(100);
    expect(contributor?.totalGrantsCompleted).toBe(0);
  });
});
