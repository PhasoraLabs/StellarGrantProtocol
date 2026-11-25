import express from "express";
import request from "supertest";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Keypair } from "@stellar/stellar-sdk";
import { Repository } from "typeorm";
import { MilestoneProof } from "../../src/entities/MilestoneProof";
import { buildMilestoneProofRouter } from "../../src/routes/milestone-proof";
import { SignatureService } from "../../src/services/signature-service";
import { NonceService } from "../../src/services/nonce-service";
import {
  MILESTONE_PROOF_CID_MAX_LENGTH,
  MILESTONE_PROOF_DESCRIPTION_MAX_LENGTH,
  MILESTONE_PROOF_NONCE_MAX_LENGTH,
  MILESTONE_PROOF_SIGNATURE_MAX_LENGTH,
} from "../../src/schemas";

describe("routes/milestone-proof input length limits", () => {
  const signatureService = new SignatureService();
  let saveMock: ReturnType<typeof vi.fn>;
  let consumeMock: ReturnType<typeof vi.fn>;
  let app: express.Express;

  beforeEach(() => {
    saveMock = vi.fn(async (data: Partial<MilestoneProof>) => ({ id: 1, ...data }));
    consumeMock = vi.fn(async () => undefined);

    const proofRepo = {
      save: saveMock,
    } as unknown as Repository<MilestoneProof>;

    const nonceService = {
      consume: consumeMock,
    } as unknown as NonceService;

    app = express();
    app.use(express.json());
    app.use("/milestone_proof", buildMilestoneProofRouter(proofRepo, signatureService, undefined, undefined, nonceService));
  });

  const buildSignedPayload = (overrides: Record<string, unknown> = {}) => {
    const keypair = Keypair.random();
    const base = {
      grantId: 1,
      milestoneIdx: 0,
      proofCid: "bafybeihk-example-proof",
      submittedBy: keypair.publicKey(),
      nonce: "nonce-12345678",
      timestamp: Date.now(),
    };
    const message = signatureService.buildIntentMessage(base);
    const signature = keypair.sign(Buffer.from(message, "utf8")).toString("base64");

    return {
      ...base,
      signature,
      ...overrides,
    };
  };

  it("accepts a valid payload within all field length limits", async () => {
    const payload = buildSignedPayload({
      description: "Milestone deliverables completed and deployed.",
    });

    const response = await request(app).post("/milestone_proof").send(payload);

    expect(response.status).toBe(201);
    expect(saveMock).toHaveBeenCalledWith(
      expect.objectContaining({
        proofCid: payload.proofCid,
        description: payload.description,
        submittedBy: payload.submittedBy,
      }),
    );
  });

  it("returns 400 when proofCid exceeds maximum length", async () => {
    const payload = buildSignedPayload({
      proofCid: "a".repeat(MILESTONE_PROOF_CID_MAX_LENGTH + 1),
    });

    const response = await request(app).post("/milestone_proof").send(payload);

    expect(response.status).toBe(400);
    expect(response.body.error).toBe("Invalid payload");
    expect(saveMock).not.toHaveBeenCalled();
  });

  it("returns 400 when description exceeds maximum length", async () => {
    const payload = buildSignedPayload({
      description: "d".repeat(MILESTONE_PROOF_DESCRIPTION_MAX_LENGTH + 1),
    });

    const response = await request(app).post("/milestone_proof").send(payload);

    expect(response.status).toBe(400);
    expect(response.body.error).toBe("Invalid payload");
    expect(saveMock).not.toHaveBeenCalled();
  });

  it("returns 400 when submittedBy exceeds maximum length", async () => {
    const payload = buildSignedPayload({
      submittedBy: "G".repeat(121),
    });

    const response = await request(app).post("/milestone_proof").send(payload);

    expect(response.status).toBe(400);
    expect(response.body.error).toBe("Invalid payload");
    expect(saveMock).not.toHaveBeenCalled();
  });

  it("returns 400 when signature exceeds maximum length", async () => {
    const payload = buildSignedPayload({
      signature: "s".repeat(MILESTONE_PROOF_SIGNATURE_MAX_LENGTH + 1),
    });

    const response = await request(app).post("/milestone_proof").send(payload);

    expect(response.status).toBe(400);
    expect(response.body.error).toBe("Invalid payload");
    expect(saveMock).not.toHaveBeenCalled();
  });

  it("returns 400 when nonce exceeds maximum length", async () => {
    const payload = buildSignedPayload({
      nonce: "n".repeat(MILESTONE_PROOF_NONCE_MAX_LENGTH + 1),
    });

    const response = await request(app).post("/milestone_proof").send(payload);

    expect(response.status).toBe(400);
    expect(response.body.error).toBe("Invalid payload");
    expect(saveMock).not.toHaveBeenCalled();
  });

  it("returns 400 when address param on GET /by-contributor/:address exceeds maximum length", async () => {
    const response = await request(app).get(`/milestone_proof/by-contributor/${"G".repeat(121)}`);

    expect(response.status).toBe(400);
    expect(response.body.error).toBe("Invalid contributor address");
  });
});
