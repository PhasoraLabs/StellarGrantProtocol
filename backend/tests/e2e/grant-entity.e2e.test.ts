import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { Grant } from "../../src/entities/Grant";
import { Milestone } from "../../src/entities/Milestone";
import { MilestoneProof } from "../../src/entities/MilestoneProof";
import { Community } from "../../src/entities/Community";
import { GrantReviewer } from "../../src/entities/GrantReviewer";
import { User } from "../../src/entities/User";
import { Role } from "../../src/entities/Role";
import { UserRole } from "../../src/entities/UserRole";

describe("Grant entity schema", () => {
  let dataSource: DataSource;

  beforeAll(async () => {
    dataSource = new DataSource({
      type: "sqljs",
      entities: [Grant, Milestone, MilestoneProof, Community, GrantReviewer, User, Role, UserRole],
      synchronize: true,
    });
    await dataSource.initialize();
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) await dataSource.destroy();
  });

  const draft = (extra: Partial<Grant> = {}) =>
    dataSource.getRepository(Grant).create({
      title: "Draft",
      recipient: "GRECIPIENT",
      totalAmount: "0",
      status: "draft",
      isDraft: true,
      ...extra,
    });

  it("generates the id when a draft is created without one", async () => {
    const repo = dataSource.getRepository(Grant);
    const first = await repo.save(draft({ owner: "GOWNER" }));
    const second = await repo.save(draft({ owner: "GOWNER" }));
    expect(first.id).toBeGreaterThan(0);
    expect(second.id).toBeGreaterThan(first.id);
  });

  it("still accepts an explicit id (on-chain sync)", async () => {
    const saved = await dataSource.getRepository(Grant).save(draft({ id: 4242, owner: "GOWNER" }));
    expect(saved.id).toBe(4242);
  });

  it("rejects a grant without an owner", async () => {
    await expect(dataSource.getRepository(Grant).save(draft())).rejects.toThrow();
  });

  it("declares owner as NOT NULL, matching the migration", () => {
    const owner = dataSource.getMetadata(Grant).findColumnWithPropertyName("owner");
    expect(owner?.isNullable).toBe(false);
  });
});
