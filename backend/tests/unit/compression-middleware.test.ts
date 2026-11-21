import express from "express";
import request from "supertest";
import { describe, expect, it } from "vitest";
import compression from "compression";

describe("compression middleware", () => {
  it("compresses responses larger than threshold", async () => {
    const app = express();
    app.use(compression({ threshold: 1024 }));
    app.get("/large", (_req, res) => {
      const largeData = "x".repeat(2048);
      res.json({ data: largeData });
    });

    const res = await request(app).get("/large").set("Accept-Encoding", "gzip");
    expect(res.status).toBe(200);
    expect(res.headers["content-encoding"]).toBe("gzip");
  });

  it("does not compress responses smaller than threshold", async () => {
    const app = express();
    app.use(compression({ threshold: 1024 }));
    app.get("/small", (_req, res) => {
      const smallData = "x".repeat(512);
      res.json({ data: smallData });
    });

    const res = await request(app).get("/small").set("Accept-Encoding", "gzip");
    expect(res.status).toBe(200);
    expect(res.headers["content-encoding"]).toBeUndefined();
  });

  it("works without Accept-Encoding header", async () => {
    const app = express();
    app.use(compression({ threshold: 1024 }));
    app.get("/any", (_req, res) => {
      res.json({ data: "test" });
    });

    const res = await request(app).get("/any");
    expect(res.status).toBe(200);
  });
});
