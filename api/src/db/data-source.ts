import "reflect-metadata";
import { DataSource } from "typeorm";
import { env } from "../config/env";
import { Activity } from "../entities/Activity";
import { AuditLog } from "../entities/AuditLog";
import { Community } from "../entities/Community";
import { Contributor } from "../entities/Contributor";
import { FeeCollection } from "../entities/FeeCollection";
import { Grant } from "../entities/Grant";
import { GrantHistory } from "../entities/GrantHistory";
import { GrantReviewer } from "../entities/GrantReviewer";
import { GrantView } from "../entities/GrantView";
import { Milestone } from "../entities/Milestone";
import { MilestoneApproval } from "../entities/MilestoneApproval";
import { MilestoneComment } from "../entities/MilestoneComment";
import { MilestoneProof } from "../entities/MilestoneProof";
import { PlatformConfig } from "../entities/PlatformConfig";
import { RateLimitLog } from "../entities/RateLimitLog";
import { ReconciliationCheckpoint } from "../entities/ReconciliationCheckpoint";
import { Report } from "../entities/Report";
import { ReputationLog } from "../entities/ReputationLog";
import { Role } from "../entities/Role";
import { User } from "../entities/User";
import { UserRole } from "../entities/UserRole";
import { UserWatchlist } from "../entities/UserWatchlist";
import { WebhookDeliveryLog } from "../entities/WebhookDeliveryLog";
import { WebhookSubscription } from "../entities/WebhookSubscription";

const entities = [
  Activity,
  AuditLog,
  Community,
  Contributor,
  FeeCollection,
  Grant,
  GrantHistory,
  GrantReviewer,
  GrantView,
  Milestone,
  MilestoneApproval,
  MilestoneComment,
  MilestoneProof,
  PlatformConfig,
  RateLimitLog,
  ReconciliationCheckpoint,
  Report,
  ReputationLog,
  Role,
  User,
  UserRole,
  UserWatchlist,
  WebhookDeliveryLog,
  WebhookSubscription,
];

export const buildDataSource = (databaseUrl = env.databaseUrl) =>
  new DataSource({
    type: databaseUrl.startsWith("sqljs") ? "sqljs" : "postgres",
    ...(databaseUrl.startsWith("sqljs")
      ? { location: databaseUrl.replace("sqljs://", ""), autoSave: false }
      : { url: databaseUrl }),
    entities,
    synchronize: true,
  });
