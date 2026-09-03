import request from "supertest";
import { beforeAll, afterAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import { env } from "../../src/config/env";
import { Milestone } from "../../src/entities/Milestone";
import { User } from "../../src/entities/User";
import { Role, RoleName } from "../../src/entities/Role";
import { UserRole } from "../../src/entities/UserRole";

describe("Communities, comments, and metrics e2e", () => {
  let dataSource: DataSource;
  const sorobanClient = new MockSorobanContractClient();

  const adminAddress = "GADMINEXAMPLEADDRESS000000000000000000000000000000000000";

  const previousAdminAddresses = [...env.adminAddresses];
  const previousMetricsUser = env.metricsBasicAuthUser;
  const previousMetricsPassword = env.metricsBasicAuthPassword;
  const previousMetricsIps = [...env.metricsAllowedIps];

  beforeAll(async () => {
    env.adminAddresses = [adminAddress];
    env.metricsBasicAuthUser = "metrics";
    env.metricsBasicAuthPassword = "secret";
    env.metricsAllowedIps = [];

    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();

    // Seed admin user with RBAC role for community operations
    const userRepo = dataSource.getRepository(User);
    const roleRepo = dataSource.getRepository(Role);
    const userRoleRepo = dataSource.getRepository(UserRole);

    // Create admin user
    const adminUser = userRepo.create({ email: "admin@test.com", stellarAddress: adminAddress });
    await userRepo.save(adminUser);

    // Create admin role (initializeDefaultRoles runs in createApp but may not have completed yet)
    let adminRole = await roleRepo.findOne({ where: { name: RoleName.ADMIN } });
    if (!adminRole) {
      adminRole = roleRepo.create({ name: RoleName.ADMIN, permissions: ["admin:all"] });
      await roleRepo.save(adminRole);
    }

    // Assign admin role to admin user
    const userRole = userRoleRepo.create({ userId: adminUser.id, roleId: adminRole.id, assignedBy: "system" });
    await userRoleRepo.save(userRole);
  });

  afterAll(async () => {
    env.adminAddresses = previousAdminAddresses;
    env.metricsBasicAuthUser = previousMetricsUser;
    env.metricsBasicAuthPassword = previousMetricsPassword;
    env.metricsAllowedIps = previousMetricsIps;

    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  it("protects /metrics with basic auth and exposes Prometheus metrics", async () => {
    const app = createApp(dataSource, sorobanClient);

    const unauthorized = await request(app).get("/metrics");
    expect(unauthorized.status).toBe(401);

    const authorized = await request(app)
      .get("/metrics")
      .set("Authorization", `Basic ${Buffer.from("metrics:secret").toString("base64")}`);

    expect(authorized.status).toBe(200);
    expect(authorized.text).toContain("stellargrant_http_requests_total");
    expect(authorized.text).toContain("stellargrant_process_cpu_user_seconds_total");
  });

  it("creates communities, associates grants, and filters grants by community", async () => {
    const app = createApp(dataSource, sorobanClient);

    await request(app).get("/grants").expect(200);

    const created = await request(app)
      .post("/communities")
      .set("x-user-address", adminAddress)
      .send({
        name: "DAO Builders",
        description: "Community for DAO-native grant programs",
        adminAddresses: [adminAddress],
        featured: true,
      });

    expect(created.status).toBe(201);
    const communityId = created.body.data.id;

    const linked = await request(app)
      .post(`/communities/${communityId}/grants/1`)
      .set("x-user-address", adminAddress);

    expect(linked.status).toBe(200);
    expect(linked.body.data.communityId).toBe(communityId);

    const communityGrants = await request(app).get(`/communities/${communityId}/grants`);
    expect(communityGrants.status).toBe(200);
    expect(communityGrants.body.data.length).toBe(1);
    expect(communityGrants.body.data[0].id).toBe(1);
    expect(Array.isArray(communityGrants.body.activity)).toBe(true);

    const filtered = await request(app).get(`/grants?communityId=${communityId}`);
    expect(filtered.status).toBe(200);
    expect(filtered.body.data.length).toBe(1);
    expect(filtered.body.data[0].id).toBe(1);
  });

  it("supports milestone comments with threading, ordering, and admin moderation", async () => {
    const app = createApp(dataSource, sorobanClient);

    await request(app).get("/grants").expect(200);

    const milestone = await dataSource.getRepository(Milestone).findOne({ where: { grantId: 1, idx: 0 } });
    expect(milestone).toBeTruthy();

    const first = await request(app)
      .post(`/milestones/${milestone!.id}/comments`)
      .send({
        content: "Please include proof-of-work details.",
        authorAddress: "GAUTHOREXAMPLEADDRESS000000000000000000000000000000000000",
      });

    expect(first.status).toBe(201);

    const second = await request(app)
      .post(`/milestones/${milestone!.id}/comments`)
      .send({
        content: "Added the details in the latest update.",
        authorAddress: "GCREATOREXAMPLEADDRESS000000000000000000000000000000000000",
        parentCommentId: first.body.data.id,
      });

    expect(second.status).toBe(201);

    const list = await request(app).get(`/milestones/${milestone!.id}/comments`);
    expect(list.status).toBe(200);
    expect(list.body.data.length).toBe(2);
    expect(list.body.data[0].id).toBe(first.body.data.id);
    expect(list.body.data[1].parentCommentId).toBe(first.body.data.id);

    const deleted = await request(app)
      .delete(`/milestones/${milestone!.id}/comments/${second.body.data.id}`)
      .set("x-admin-address", adminAddress);

    expect(deleted.status).toBe(200);

    const listAfterDelete = await request(app).get(`/milestones/${milestone!.id}/comments`);
    expect(listAfterDelete.status).toBe(200);
    expect(listAfterDelete.body.data.length).toBe(1);
  });
});
