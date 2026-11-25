import express from "express";
import request from "supertest";
import { Keypair } from "@stellar/stellar-sdk";
import { describe, expect, it } from "vitest";
import { buildSignedUserAuth } from "../../src/middlewares/signed-user-auth";
import { NonceReplayError, NonceService } from "../../src/services/nonce-service";
import { SignatureService } from "../../src/services/signature-service";

const errorHandler = (err: any, _req: any, res: any, _next: any) =>
  res.status(err.statusCode ?? err.status ?? 500).json({ error: err.message });

const signatureService = new SignatureService();

const memoryNonceService = () => {
  const seen = new Set<string>();
  return {
    consume: async (nonce: string) => {
      if (seen.has(nonce)) throw new NonceReplayError(nonce);
      seen.add(nonce);
    },
  } as unknown as NonceService;
};

const signedUserHeaders = (keypair: Keypair, method: string, path: string, timestamp?: number) => {
  const nonce = Math.random().toString(36).substring(7);
  const ts = timestamp ?? Date.now();
  const action = `${method}:${path}`;
  const message = signatureService.buildUserIntentMessage({
    address: keypair.publicKey(),
    nonce,
    timestamp: ts,
    action,
  });
  const signature = keypair.sign(Buffer.from(message, "utf8")).toString("base64");

  return {
    "x-stellar-address": keypair.publicKey(),
    "x-stellar-signature": signature,
    "x-stellar-nonce": nonce,
    "x-stellar-timestamp": ts.toString(),
  };
};

describe("signed-user-auth middleware - expired signatures", () => {
  const user = Keypair.random();
  const build = () => {
    const auth = buildSignedUserAuth(signatureService, memoryNonceService());
    const app = express();
    app.use(express.json());
    app.use(auth);
    app.get("/protected", (_req, res) => res.json({ ok: true }));
    app.use(errorHandler);
    return app;
  };

  it("rejects a request with an expired timestamp (older than 5 minutes)", () => {
    const app = build();
    const expiredTimestamp = Date.now() - (5 * 60 * 1000) - 1000; // 5 minutes + 1 second ago
    const headers = signedUserHeaders(user, "GET", "/protected", expiredTimestamp);

    return request(app).get("/protected").set(headers).expect(401);
  });

  it("rejects a request with a future timestamp (more than 5 minutes ahead)", () => {
    const app = build();
    const futureTimestamp = Date.now() + (5 * 60 * 1000) + 1000; // 5 minutes + 1 second in future
    const headers = signedUserHeaders(user, "GET", "/protected", futureTimestamp);

    return request(app).get("/protected").set(headers).expect(401);
  });

  it("accepts a request with a timestamp at the 5 minute boundary", () => {
    const app = build();
    const boundaryTimestamp = Date.now() - (5 * 60 * 1000); // Exactly 5 minutes ago
    const headers = signedUserHeaders(user, "GET", "/protected", boundaryTimestamp);

    return request(app).get("/protected").set(headers).expect(200);
  });

  it("accepts a request with a valid timestamp within the allowed window", () => {
    const app = build();
    const validTimestamp = Date.now() - (4 * 60 * 1000); // 4 minutes ago
    const headers = signedUserHeaders(user, "GET", "/protected", validTimestamp);

    return request(app).get("/protected").set(headers).expect(200);
  });
});

describe("signed-user-auth middleware - nonce reuse", () => {
  const user = Keypair.random();
  const build = () => {
    const auth = buildSignedUserAuth(signatureService, memoryNonceService());
    const app = express();
    app.use(express.json());
    app.use(auth);
    app.get("/protected", (_req, res) => res.json({ ok: true }));
    app.use(errorHandler);
    return app;
  };

  it("rejects a replayed nonce", async () => {
    const app = build();
    const headers = signedUserHeaders(user, "GET", "/protected");

    const firstRes = await request(app).get("/protected").set(headers);
    expect(firstRes.status).toBe(200);

    const secondRes = await request(app).get("/protected").set(headers);
    expect(secondRes.status).toBe(401);
    expect(secondRes.body.error).toMatch(/nonce/i);
  });

  it("accepts requests with different nonces", async () => {
    const app = build();
    const headers1 = signedUserHeaders(user, "GET", "/protected");
    const headers2 = signedUserHeaders(user, "GET", "/protected");

    const res1 = await request(app).get("/protected").set(headers1);
    expect(res1.status).toBe(200);

    const res2 = await request(app).get("/protected").set(headers2);
    expect(res2.status).toBe(200);
  });

  it("rejects a request with a previously used nonce from a different action", async () => {
    const app = build();
    const headers = signedUserHeaders(user, "GET", "/protected");

    await request(app).get("/protected").set(headers).expect(200);

    // Reuse the same nonce for a different path (middleware will still reject due to nonce reuse)
    const secondRes = await request(app).get("/other-path").set(headers);
    expect(secondRes.status).toBe(401);
  });
});

describe("signed-user-auth middleware - missing headers", () => {
  const user = Keypair.random();
  const build = () => {
    const auth = buildSignedUserAuth(signatureService, memoryNonceService());
    const app = express();
    app.use(express.json());
    app.use(auth);
    app.get("/protected", (_req, res) => res.json({ ok: true }));
    app.use(errorHandler);
    return app;
  };

  it("rejects a request with missing x-stellar-address header", () => {
    const app = build();
    const headers = signedUserHeaders(user, "GET", "/protected");
    delete (headers as Record<string, string>)["x-stellar-address"];

    return request(app).get("/protected").set(headers).expect(401);
  });

  it("rejects a request with missing x-stellar-signature header", () => {
    const app = build();
    const headers = signedUserHeaders(user, "GET", "/protected");
    delete (headers as Record<string, string>)["x-stellar-signature"];

    return request(app).get("/protected").set(headers).expect(401);
  });

  it("rejects a request with missing x-stellar-nonce header", () => {
    const app = build();
    const headers = signedUserHeaders(user, "GET", "/protected");
    delete (headers as Record<string, string>)["x-stellar-nonce"];

    return request(app).get("/protected").set(headers).expect(401);
  });

  it("rejects a request with missing x-stellar-timestamp header", () => {
    const app = build();
    const headers = signedUserHeaders(user, "GET", "/protected");
    delete (headers as Record<string, string>)["x-stellar-timestamp"];

    return request(app).get("/protected").set(headers).expect(401);
  });
});

describe("signed-user-auth middleware - optional mode", () => {
  const user = Keypair.random();
  const build = () => {
    const auth = buildSignedUserAuth(signatureService, memoryNonceService(), { optional: true });
    const app = express();
    app.use(express.json());
    app.use(auth);
    app.get("/public", (req, res) => {
      const hasUser = (req as any).user !== undefined;
      res.json({ hasUser });
    });
    app.use(errorHandler);
    return app;
  };

  it("allows requests without authentication headers in optional mode", async () => {
    const app = build();
    const res = await request(app).get("/public");
    expect(res.status).toBe(200);
    expect(res.body.hasUser).toBe(false);
  });

  it("accepts valid authentication in optional mode", async () => {
    const app = build();
    const headers = signedUserHeaders(user, "GET", "/public");
    const res = await request(app).get("/public").set(headers);
    expect(res.status).toBe(200);
    expect(res.body.hasUser).toBe(true);
  });

  it("rejects invalid authentication in optional mode", async () => {
    const app = build();
    const headers = signedUserHeaders(user, "GET", "/public");
    headers["x-stellar-signature"] = "invalid";

    const res = await request(app).get("/public").set(headers);
    expect(res.status).toBe(401);
  });
});
