import { Keypair } from "@stellar/stellar-sdk";
import request from "supertest";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { Grant } from "../../src/entities/Grant";
import { UserWatchlist } from "../../src/entities/UserWatchlist";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";
import { signedUserHeaders } from "../helpers/sign-user";

describe("watchlist routes", () => {
  let dataSource: DataSource;
  let app: ReturnType<typeof createApp>;

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();

    app = createApp(dataSource, new MockSorobanContractClient());
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  const createGrant = async () =>
    dataSource.getRepository(Grant).save({
      title: "Watchlist integration grant",
      status: "active",
      owner: "GOWNER000000000000000000000000000000000000000000000000000",
      recipient: "GRECIPIENT00000000000000000000000000000000000000000000000",
      totalAmount: "100",
      isDraft: false,
    });

  const signedRequest = (
    method: "get" | "post" | "delete",
    path: string,
    keypair: Keypair,
  ) => request(app)[method](path).set(signedUserHeaders(keypair, method, path));

  it("returns an empty list for a user with no watchlist entries", async () => {
    const keypair = Keypair.random();
    const response = await signedRequest("get", "/watchlist", keypair);

    expect(response.status).toBe(200);
    expect(response.body).toEqual({ data: [] });
  });

  it("adds a grant to the authenticated user's watchlist", async () => {
    const keypair = Keypair.random();
    const grant = await createGrant();
    const response = await signedRequest(
      "post",
      `/watchlist/${grant.id}`,
      keypair,
    );

    expect(response.status).toBe(201);
    expect(response.body).toEqual({ ok: true });

    const listResponse = await signedRequest("get", "/watchlist", keypair);
    expect(listResponse.status).toBe(200);
    expect(listResponse.body.data.map((item: Grant) => item.id)).toContain(
      grant.id,
    );
  });

  it("rejects adding a grant that does not exist", async () => {
    const keypair = Keypair.random();
    const response = await signedRequest(
      "post",
      "/watchlist/999999",
      keypair,
    );

    expect(response.status).toBe(404);
    expect(response.body).toEqual({ error: "Grant not found" });
  });

  it("returns a conflict when the same grant is added twice", async () => {
    const keypair = Keypair.random();
    const grant = await createGrant();
    const path = `/watchlist/${grant.id}`;

    const firstResponse = await signedRequest("post", path, keypair);
    const duplicateResponse = await signedRequest("post", path, keypair);

    expect(firstResponse.status).toBe(201);
    expect(duplicateResponse.status).toBe(409);
    expect(duplicateResponse.body).toEqual({
      error: "Grant already in watchlist",
    });
  });

  it("removes a grant from the authenticated user's watchlist", async () => {
    const keypair = Keypair.random();
    const grant = await createGrant();
    const path = `/watchlist/${grant.id}`;

    await signedRequest("post", path, keypair);
    const response = await signedRequest("delete", path, keypair);

    expect(response.status).toBe(200);
    expect(response.body).toEqual({ ok: true });
    expect(
      await dataSource.getRepository(UserWatchlist).findOneBy({
        address: keypair.publicKey(),
        grantId: grant.id,
      }),
    ).toBeNull();
  });

  it("returns not found when removing a grant not in the watchlist", async () => {
    const keypair = Keypair.random();
    const grant = await createGrant();
    const response = await signedRequest(
      "delete",
      `/watchlist/${grant.id}`,
      keypair,
    );

    expect(response.status).toBe(404);
    expect(response.body).toEqual({ error: "Watchlist entry not found" });
  });

  it("rejects invalid grant IDs", async () => {
    const keypair = Keypair.random();
    const response = await signedRequest(
      "post",
      "/watchlist/not-a-grant-id",
      keypair,
    );

    expect(response.status).toBe(400);
    expect(response.body.error).toBe("Validation failed");
  });
});
