import express from "express";
import request from "supertest";
import { Keypair } from "@stellar/stellar-sdk";
import { describe, expect, it, vi } from "vitest";
import { buildAdminMiddleware } from "../../src/middlewares/admin-middleware";
import { buildSignedUserAuth } from "../../src/middlewares/signed-user-auth";
import { buildAuthMiddleware } from "../../src/middlewares/auth-middleware";
import { createRbacMiddleware } from "../../src/middlewares/rbac-middleware";
import { buildMyDonationsRouter } from "../../src/routes/my-donations";
import { buildGrantDraftsRouter } from "../../src/routes/grant-drafts";
import { NonceReplayError, NonceService } from "../../src/services/nonce-service";
import { SignatureService } from "../../src/services/signature-service";
import { env } from "../../src/config/env";
import { signedUserHeaders } from "../helpers/sign-user";

const errorHandler = (err: any, _req: any, res: any, _next: any) =>
  res.status(err.statusCode ?? err.status ?? 500).json({ error: err.message });

const signatureService = new SignatureService();

/** In-memory stand-in for the used_nonces table with the same replay semantics. */
const memoryNonceService = () => {
  const seen = new Set<string>();
  return {
    consume: async (nonce: string) => {
      if (seen.has(nonce)) throw new NonceReplayError(nonce);
      seen.add(nonce);
    },
  } as unknown as NonceService;
};

const adminHeaders = (admin: Keypair, method: string, path: string, nonce = "nonce-1") => {
  const timestamp = Date.now();
  const action = `${method}:${path}`;
  const message = signatureService.buildAdminIntentMessage({ address: admin.publicKey(), nonce, timestamp, action });
  return {
    "x-admin-address": admin.publicKey(),
    "x-admin-nonce": nonce,
    "x-admin-timestamp": String(timestamp),
    "x-admin-signature": admin.sign(Buffer.from(message, "utf8")).toString("base64"),
  };
};

describe("admin middleware replay protection (#1204)", () => {
  const admin = Keypair.random();
  const build = () => {
    env.adminAddresses.push(admin.publicKey());
    const app = express();
    app.use(buildAdminMiddleware(signatureService, memoryNonceService()));
    app.post("/admin/thing", (_req, res) => res.json({ ok: true }));
    return app;
  };

  it("accepts a signed request once and rejects the identical replay", async () => {
    const app = build();
    const headers = adminHeaders(admin, "POST", "/admin/thing");

    await request(app).post("/admin/thing").set(headers).expect(200);
    const replay = await request(app).post("/admin/thing").set(headers);

    expect(replay.status).toBe(401);
    expect(replay.body.error).toMatch(/nonce/i);
  });

  it("accepts a fresh nonce", async () => {
    const app = build();
    await request(app).post("/admin/thing").set(adminHeaders(admin, "POST", "/admin/thing", "a")).expect(200);
    await request(app).post("/admin/thing").set(adminHeaders(admin, "POST", "/admin/thing", "b")).expect(200);
  });
});

describe("rbac middleware identity (#1205)", () => {
  it("rejects a request that only sets x-user-address", async () => {
    const rbacService = { userHasPermissionByAddress: vi.fn().mockResolvedValue(true) } as any;
    const { requirePermission } = createRbacMiddleware(rbacService);
    const app = express();
    app.get("/protected", requirePermission("admin:all" as any), (_req, res) => res.json({ ok: true }));

    const res = await request(app).get("/protected").set("x-user-address", "GADMINADDRESS");

    expect(res.status).toBe(401);
    expect(rbacService.userHasPermissionByAddress).not.toHaveBeenCalled();
  });
});

describe("GET /grants/users/:address/drafts (#1206)", () => {
  const owner = Keypair.random();
  const other = Keypair.random();
  const build = () => {
    const find = vi.fn().mockResolvedValue([]);
    const dataSource = { getRepository: () => ({ find }) } as any;
    const auth = buildSignedUserAuth(signatureService, memoryNonceService());
    const app = express();
    app.use(express.json());
    app.use("/grants", buildGrantDraftsRouter(dataSource, {} as any, auth));
    app.use(errorHandler);
    return { app, find };
  };
  const path = (a: Keypair) => `/grants/users/${a.publicKey()}/drafts`;

  it("rejects an unauthenticated request", async () => {
    const { app, find } = build();
    await request(app).get(path(owner)).expect(401);
    expect(find).not.toHaveBeenCalled();
  });

  it("rejects a spoofed address header without a valid signature", async () => {
    const { app, find } = build();
    await request(app).get(path(owner)).set("x-stellar-address", owner.publicKey()).expect(401);
    expect(find).not.toHaveBeenCalled();
  });

  it("rejects a valid signer asking for another address's drafts", async () => {
    const { app, find } = build();
    const res = await request(app).get(path(owner)).set(signedUserHeaders(other, "GET", path(owner)));
    expect(res.status).toBe(403);
    expect(find).not.toHaveBeenCalled();
  });

  it("lists drafts for the proven owner", async () => {
    const { app } = build();
    await request(app).get(path(owner)).set(signedUserHeaders(owner, "GET", path(owner))).expect(200);
  });
});

describe("GET /my-donations (#1207)", () => {
  const funder = Keypair.random();
  const build = () => {
    const find = vi.fn().mockResolvedValue([]);
    const dataSource = { getRepository: () => ({ find }) } as any;
    const auth = buildSignedUserAuth(signatureService, memoryNonceService());
    const app = express();
    app.use("/my-donations", buildMyDonationsRouter(dataSource, auth));
    app.use(errorHandler);
    return { app, find };
  };

  it("does not trust a spoofed x-stellar-address header", async () => {
    const { app, find } = build();
    await request(app).get("/my-donations").set("x-stellar-address", funder.publicKey()).expect(401);
    expect(find).not.toHaveBeenCalled();
  });

  it("rejects a signature made by a different key", async () => {
    const { app, find } = build();
    const headers = { ...signedUserHeaders(Keypair.random(), "GET", "/my-donations"), "x-stellar-address": funder.publicKey() };
    await request(app).get("/my-donations").set(headers).expect(401);
    expect(find).not.toHaveBeenCalled();
  });

  it("returns donations queried by the verified caller's address only", async () => {
    const { app, find } = build();
    await request(app).get("/my-donations").set(signedUserHeaders(funder, "GET", "/my-donations")).expect(200);
    expect(find.mock.calls[0][0].where.funderAddress).toBe(funder.publicKey());
  });

  it("rejects a replayed signed request", async () => {
    const { app } = build();
    const headers = signedUserHeaders(funder, "GET", "/my-donations");
    await request(app).get("/my-donations").set(headers).expect(200);
    await request(app).get("/my-donations").set(headers).expect(401);
  });
});

describe("authMiddleware signature verification (#1194)", () => {
  const user = Keypair.random();
  const build = () => {
    const auth = buildAuthMiddleware(signatureService, memoryNonceService());
    const app = express();
    app.use(express.json());
    app.use(auth);
    app.get("/protected", (_req, res) => res.json({ ok: true }));
    app.use(errorHandler);
    return app;
  };

  it("rejects a request with missing authentication headers", async () => {
    const app = build();
    await request(app).get("/protected").expect(401);
  });

  it("rejects a forged header without a valid signature", async () => {
    const app = build();
    await request(app)
      .get("/protected")
      .set("x-stellar-address", user.publicKey())
      .expect(401);
  });

  it("rejects a request with valid signature but wrong address in header", async () => {
    const app = build();
    const headers = signedUserHeaders(user, "GET", "/protected");
    await request(app)
      .get("/protected")
      .set("x-stellar-address", Keypair.random().publicKey())
      .set("x-stellar-signature", headers["x-stellar-signature"])
      .set("x-stellar-nonce", headers["x-stellar-nonce"])
      .set("x-stellar-timestamp", headers["x-stellar-timestamp"])
      .expect(401);
  });

  it("accepts a legitimately signed request", async () => {
    const app = build();
    await request(app)
      .get("/protected")
      .set(signedUserHeaders(user, "GET", "/protected"))
      .expect(200);
  });

  it("rejects a replayed signature", async () => {
    const app = build();
    const headers = signedUserHeaders(user, "GET", "/protected");
    await request(app).get("/protected").set(headers).expect(200);
    await request(app).get("/protected").set(headers).expect(401);
  });
});
