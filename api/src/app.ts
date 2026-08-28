import express from "express";
import helmet from "helmet";
import cors from "cors";
import morgan from "morgan";
import { DataSource } from "typeorm";
import { Grant } from "./entities/Grant";
import { MilestoneProof } from "./entities/MilestoneProof";
import { Contributor } from "./entities/Contributor";
import { AuditLog } from "./entities/AuditLog";
import { Community } from "./entities/Community";
import { Activity } from "./entities/Activity";
import { buildGrantRouter } from "./routes/grants";
import { buildMilestoneProofRouter } from "./routes/milestone-proof";
import { buildSearchRouter } from "./routes/search";
import { buildLeaderboardRouter } from "./routes/leaderboard";
import { buildProfilesRouter } from "./routes/profiles";
import { buildMyDonationsRouter } from "./routes/my-donations";
import { buildCommunitiesRouter } from "./routes/communities";
import { buildNotificationsRouter } from "./routes/notifications";
import { buildAdminRouter } from "./routes/admin";
import { buildAdminMiddleware } from "./middlewares/admin-middleware";
import { GrantSyncService } from "./services/grant-sync-service";
import { SignatureService } from "./services/signature-service";
import { LeaderboardService } from "./services/leaderboard-service";
import { ResponseCacheService } from "./services/response-cache";
import { RbacService } from "./services/rbac-service";
import { metricsService } from "./services/metrics-service";
import { env } from "./config/env";
import { ReconciliationService } from "./services/reconciliation-service";
import { SorobanContractClient } from "./soroban/types";
import { AppError } from "./utils/errors";

import { User } from "./entities/User";
import { Role } from "./entities/Role";
import { UserRole } from "./entities/UserRole";
import { GrantReviewer } from "./entities/GrantReviewer";
import { MilestoneApproval } from "./entities/MilestoneApproval";
import { Milestone } from "./entities/Milestone";
import { MilestoneComment } from "./entities/MilestoneComment";
import { WebhookDispatcher } from "./services/webhook-dispatcher";
import { notificationService } from "./services/notification-service";
import { buildUserRouter } from "./routes/users";
import { buildGrantReviewerRouter } from "./routes/grant-reviewers";
import { buildMilestoneApprovalNotifyRouter } from "./routes/milestone-approvals-notify";
import { buildMilestoneCommentsRouter } from "./routes/milestone-comments";
import { buildDisputesRouter, buildGrantDisputesRouter } from "./routes/disputes";
import { buildMilestoneAppealsRouter } from "./routes/milestone-appeals";
import { buildGrantDraftsRouter } from "./routes/grant-drafts";
import { buildContributorsRouter } from "./routes/contributors";

export const createApp = (dataSource: DataSource, sorobanClient: SorobanContractClient) => {
  const app = express();
  app.use(helmet());
  app.use(cors());
  app.use(morgan("tiny"));
  app.use(express.json());

  const grantRepo = dataSource.getRepository(Grant);
  const proofRepo = dataSource.getRepository(MilestoneProof);
  const contributorRepo = dataSource.getRepository(Contributor);
  const auditLogRepo = dataSource.getRepository(AuditLog);
  const communityRepo = dataSource.getRepository(Community);
  const activityRepo = dataSource.getRepository(Activity);
  const userRepo = dataSource.getRepository(User);
  const roleRepo = dataSource.getRepository(Role);
  const userRoleRepo = dataSource.getRepository(UserRole);
  const reviewerRepo = dataSource.getRepository(GrantReviewer);
  const approvalRepo = dataSource.getRepository(MilestoneApproval);
  const milestoneRepo = dataSource.getRepository(Milestone);
  const commentsRepo = dataSource.getRepository(MilestoneComment);

  const grantSyncService = new GrantSyncService(dataSource, sorobanClient);
  const signatureService = new SignatureService();
  const leaderboardService = new LeaderboardService(dataSource);
  const responseCacheService = new ResponseCacheService();
  const rbacService = new RbacService(userRepo, roleRepo, userRoleRepo);
  const adminMiddleware = buildAdminMiddleware(signatureService);
  const reconciliationService = new ReconciliationService(dataSource, sorobanClient, grantSyncService);
  const webhookDispatcher = new WebhookDispatcher(dataSource);

  app.get("/health", (_req, res) => res.json({ ok: true }));
  app.use("/grants", buildGrantRouter(grantRepo, grantSyncService));
  app.use("/grants", buildGrantDraftsRouter(dataSource, webhookDispatcher));
  app.use("/milestone_proof", buildMilestoneProofRouter(proofRepo, signatureService, grantRepo, userRepo));
  app.use("/search", buildSearchRouter(dataSource));
  app.use("/leaderboard", buildLeaderboardRouter(leaderboardService));
  app.use("/profiles", buildProfilesRouter(contributorRepo, grantRepo));
  app.use("/my-donations", buildMyDonationsRouter(dataSource));
  app.use("/communities", buildCommunitiesRouter(communityRepo, grantRepo, activityRepo, rbacService, webhookDispatcher));
  app.use("/notifications", buildNotificationsRouter(contributorRepo));
  app.use("/contributors", buildContributorsRouter(contributorRepo, grantRepo, proofRepo, activityRepo));
  app.use("/users", buildUserRouter(userRepo));
  app.use("/grant_reviewers", buildGrantReviewerRouter(reviewerRepo));
  app.use("/milestone_approvals_notify", buildMilestoneApprovalNotifyRouter(approvalRepo, grantRepo, userRepo, webhookDispatcher));
  app.use("/disputes", buildDisputesRouter(dataSource, rbacService, webhookDispatcher));
  app.use("/", buildGrantDisputesRouter(dataSource));
  app.use("/", buildMilestoneAppealsRouter(dataSource, webhookDispatcher, notificationService));
  app.use("/", buildMilestoneCommentsRouter(milestoneRepo, commentsRepo, reviewerRepo));
  app.get("/metrics", async (req, res, next) => {
    try {
      if (env.metricsAllowedIps.length > 0 && !env.metricsAllowedIps.includes(req.ip)) {
        res.status(403).json({ error: "Forbidden" });
        return;
      }

      const auth = String(req.header("authorization") ?? "");
      if (!auth.startsWith("Basic ")) {
        res.set("WWW-Authenticate", "Basic realm=\"metrics\"");
        res.status(401).json({ error: "Missing authorization" });
        return;
      }

      const decoded = Buffer.from(auth.replace(/^Basic\s+/i, ""), "base64").toString("utf8");
      const [user, pass] = decoded.split(":", 2);
      if (user !== env.metricsBasicAuthUser || pass !== env.metricsBasicAuthPassword) {
        res.set("WWW-Authenticate", "Basic realm=\"metrics\"");
        res.status(401).json({ error: "Invalid credentials" });
        return;
      }

      const metricsText = await metricsService.getMetricsText();
      res.set("Content-Type", metricsService.getContentType());
      res.send(metricsText);
    } catch (error) {
      next(error);
    }
  });
  app.use("/admin", adminMiddleware, buildAdminRouter(grantSyncService, contributorRepo, auditLogRepo, responseCacheService, reconciliationService));

  app.use((err: unknown, _req: express.Request, res: express.Response, _next: express.NextFunction) => {
    if (err instanceof AppError) {
      res.status(err.statusCode).json(err.toJSON());
      return;
    }
    const message = err instanceof Error ? err.message : "Internal server error";
    res.status(500).json({ error: message });
  });

  return app;
};
