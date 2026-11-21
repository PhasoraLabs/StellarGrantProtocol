import { describe, it, expect, vi, beforeEach } from "vitest";
import express from "express";
import request from "supertest";
import {
  buildIpfsPresignRouter,
  DEFAULT_MAX_FILE_SIZE,
} from "../../src/routes/ipfs-presign";
import type { SignatureService } from "../../src/services/signature-service";
import type { NonceService } from "../../src/services/nonce-service";
import type { IpfsService } from "../../src/services/ipfs-service";

describe("routes/ipfs-presign - max file size validation (#1430)", () => {
  const TEST_ADDRESS = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

  let mockSignatureService: Partial<SignatureService>;
  let mockNonceService: Partial<NonceService>;
  let mockIpfsService: Partial<IpfsService>;

  beforeEach(() => {
    mockSignatureService = {
      verifyUser: vi.fn().mockReturnValue(true),
    };
    mockNonceService = {
      consume: vi.fn().mockResolvedValue(undefined),
    };
    mockIpfsService = {
      generatePresignedKey: vi.fn().mockResolvedValue({
        uploadUrl: "https://api.pinata.cloud/pinning/pinFileToIPFS",
        jwt: "mock-jwt-token",
        expiresIn: 900,
        maxSize: 10 * 1024 * 1024,
      }),
      verifyCid: vi.fn().mockResolvedValue(true),
    };
  });

  function createApp(options?: { maxFileSize?: number; unauthenticated?: boolean }) {
    const app = express();
    app.use(express.json());

    const authMiddleware = (req: any, res: any, next: any) => {
      if (options?.unauthenticated) {
        res.status(401).json({ error: "Missing authentication headers" });
        return;
      }
      req.user = { stellarAddress: TEST_ADDRESS };
      next();
    };

    const router = buildIpfsPresignRouter(
      mockSignatureService as SignatureService,
      mockNonceService as NonceService,
      {
        ...options,
        authMiddleware,
        ipfsServiceInstance: mockIpfsService as IpfsService,
      },
    );

    app.use("/ipfs", router);
    return app;
  }

  it("accepts requests without declared size and returns configured maxSize", async () => {
    const app = createApp();
    const res = await request(app).post("/ipfs/presign").send({});

    expect(res.status).toBe(200);
    expect(res.body.data.jwt).toBe("mock-jwt-token");
    expect(res.body.data.maxSize).toBe(DEFAULT_MAX_FILE_SIZE);
  });

  it("accepts requests with fileSize within the allowed limit", async () => {
    const app = createApp();
    const res = await request(app)
      .post("/ipfs/presign")
      .send({ fileSize: 5 * 1024 * 1024 });

    expect(res.status).toBe(200);
    expect(res.body.data.jwt).toBe("mock-jwt-token");
    expect(mockIpfsService.generatePresignedKey).toHaveBeenCalledWith(TEST_ADDRESS);
  });

  it("accepts requests with size alias within the allowed limit", async () => {
    const app = createApp();
    const res = await request(app)
      .post("/ipfs/presign")
      .send({ size: 1024 });

    expect(res.status).toBe(200);
    expect(res.body.data.jwt).toBe("mock-jwt-token");
  });

  it("rejects requests when declared fileSize exceeds default max (10 MB)", async () => {
    const app = createApp();
    const excessiveSize = DEFAULT_MAX_FILE_SIZE + 1;
    const res = await request(app)
      .post("/ipfs/presign")
      .send({ fileSize: excessiveSize });

    expect(res.status).toBe(400);
    expect(res.body.error).toContain("exceeds maximum allowed size");
    expect(res.body.maxSize).toBe(DEFAULT_MAX_FILE_SIZE);
    expect(mockIpfsService.generatePresignedKey).not.toHaveBeenCalled();
  });

  it("rejects requests when declared size alias exceeds default max", async () => {
    const app = createApp();
    const excessiveSize = 20 * 1024 * 1024;
    const res = await request(app)
      .post("/ipfs/presign")
      .send({ size: excessiveSize });

    expect(res.status).toBe(400);
    expect(res.body.error).toContain("exceeds maximum allowed size");
    expect(mockIpfsService.generatePresignedKey).not.toHaveBeenCalled();
  });

  it("honors custom maxFileSize passed via options", async () => {
    const customMax = 2 * 1024 * 1024; // 2 MB
    const app = createApp({ maxFileSize: customMax });

    // Under limit
    const okRes = await request(app)
      .post("/ipfs/presign")
      .send({ fileSize: 1024 * 1024 });
    expect(okRes.status).toBe(200);
    expect(okRes.body.data.maxSize).toBe(customMax);

    // Over limit
    const overRes = await request(app)
      .post("/ipfs/presign")
      .send({ fileSize: customMax + 10 });
    expect(overRes.status).toBe(400);
    expect(overRes.body.error).toContain(`exceeds maximum allowed size of ${customMax} bytes`);
  });

  it("honors IPFS_MAX_FILE_SIZE_BYTES environment variable when options are omitted", async () => {
    const originalEnv = process.env.IPFS_MAX_FILE_SIZE_BYTES;
    try {
      process.env.IPFS_MAX_FILE_SIZE_BYTES = "1048576"; // 1 MB
      const authMiddleware = (req: any, _res: any, next: any) => {
        req.user = { stellarAddress: TEST_ADDRESS };
        next();
      };
      const app = express();
      app.use(express.json());

      const router = buildIpfsPresignRouter(
        mockSignatureService as SignatureService,
        mockNonceService as NonceService,
        {
          ipfsServiceInstance: mockIpfsService as IpfsService,
          authMiddleware,
        },
      );
      app.use("/ipfs", router);

      const res = await request(app)
        .post("/ipfs/presign")
        .send({ fileSize: 2 * 1024 * 1024 });

      expect(res.status).toBe(400);
      expect(res.body.error).toContain("exceeds maximum allowed size of 1048576 bytes");
    } finally {
      process.env.IPFS_MAX_FILE_SIZE_BYTES = originalEnv;
    }
  });

  it("rejects unauthenticated requests", async () => {
    const app = createApp({ unauthenticated: true });
    const res = await request(app).post("/ipfs/presign").send({});
    expect(res.status).toBe(401);
  });

  it("rejects non-numeric fileSize parameter", async () => {
    const app = createApp();
    const res = await request(app)
      .post("/ipfs/presign")
      .send({ fileSize: "huge" });

    expect(res.status).toBe(400);
    expect(res.body.error).toContain("fileSize must be a number");
  });

  it("rejects negative or zero fileSize", async () => {
    const app = createApp();
    const res = await request(app)
      .post("/ipfs/presign")
      .send({ fileSize: -500 });

    expect(res.status).toBe(400);
    expect(res.body.error).toContain("fileSize must be a positive number");
  });

  it("validates size declared in query string", async () => {
    const app = createApp({ maxFileSize: 1000 });
    const res = await request(app)
      .post("/ipfs/presign?fileSize=2000")
      .send({});

    expect(res.status).toBe(400);
    expect(res.body.error).toContain("exceeds maximum allowed size of 1000 bytes");
  });
});
