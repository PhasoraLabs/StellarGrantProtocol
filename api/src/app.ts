import express from "express";
import helmet from "helmet";
import { DataSource } from "typeorm";
import { Grant } from "./entities/Grant";
import { MilestoneProof } from "./entities/MilestoneProof";
import { buildGrantRouter } from "./routes/grants";
import { buildMilestoneProofRouter } from "./routes/milestone-proof";
import { GrantSyncService } from "./services/grant-sync-service";
import { SignatureService } from "./services/signature-service";
import { SorobanContractClient } from "./soroban/types";
import { buildLeaderboardRouter } from "./routes/leaderboard";
import { buildSearchRouter } from "./routes/search";

export const createApp = (dataSource: DataSource, sorobanClient: SorobanContractClient) => {
  const app = express();
  app.use(helmet());
  app.use(express.json());

  const grantRepo = dataSource.getRepository(Grant);
  const proofRepo = dataSource.getRepository(MilestoneProof);
  const grantSyncService = new GrantSyncService(dataSource, sorobanClient);
  const signatureService = new SignatureService();

  app.get("/health", (_req, res) => res.json({ ok: true }));
  app.use("/grants", buildGrantRouter(grantRepo, grantSyncService));
  app.use("/leaderboard", buildLeaderboardRouter(grantRepo));
  app.use("/search", buildSearchRouter(grantRepo));
  app.use("/milestone_proof", buildMilestoneProofRouter(proofRepo, signatureService));

  app.use((err: unknown, _req: express.Request, res: express.Response, _next: express.NextFunction) => {
    const message = err instanceof Error ? err.message : "Internal server error";
    res.status(500).json({ error: message });
  });

  return app;
};
