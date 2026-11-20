import request from "supertest";
import { beforeAll, afterAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { Keypair } from "@stellar/stellar-sdk";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";

describe("Profiles API e2e", () => {
  let dataSource: DataSource;
  const sorobanClient = new MockSorobanContractClient();

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) await dataSource.destroy();
  });

  it("allows a user to save and retrieve profile metadata with a valid signature", async () => {
    const app = createApp(dataSource, sorobanClient);
    const keypair = Keypair.random();
    const timestamp = Date.now();
    const nonce = "nonce-profile-123456";
    const patch = { bio: "Hello, Stellar!", githubUrl: "https://github.com/example-user" };

    const message = [
      "stellargrant:profile_update:v1",
      keypair.publicKey(),
      nonce,
      timestamp,
      "PATCH:/profiles/me",
      JSON.stringify(patch),
    ].join("|");
    const signature = keypair.sign(Buffer.from(message, "utf8")).toString("base64");

    const patchRes = await request(app).patch("/profiles/me").send({
      address: keypair.publicKey(),
      nonce,
      timestamp,
      signature,
      ...patch,
    });
    expect(patchRes.status).toBe(200);
    expect(patchRes.body.data.address).toBe(keypair.publicKey());
    expect(patchRes.body.data.bio).toBe("Hello, Stellar!");
    expect(patchRes.body.data.githubUrl).toBe("https://github.com/example-user");

    const getRes = await request(app).get(`/profiles/${keypair.publicKey()}`);
    expect(getRes.status).toBe(200);
    expect(getRes.body.data.address).toBe(keypair.publicKey());
    expect(getRes.body.data.bio).toBe("Hello, Stellar!");
  });

  it("rejects attempts to update someone else's profile (signature mismatch)", async () => {
    const app = createApp(dataSource, sorobanClient);
    const signer = Keypair.random();
    const victim = Keypair.random();
    const timestamp = Date.now();
    const nonce = "nonce-profile-abcdef";
    const patch = { bio: "I am not the owner" };

    const message = [
      "stellargrant:profile_update:v1",
      victim.publicKey(),
      nonce,
      timestamp,
      "PATCH:/profiles/me",
      JSON.stringify(patch),
    ].join("|");
    const signature = signer.sign(Buffer.from(message, "utf8")).toString("base64");

    const res = await request(app).patch("/profiles/me").send({
      address: victim.publicKey(),
      nonce,
      timestamp,
      signature,
      ...patch,
    });
    expect(res.status).toBe(401);
  });
});

