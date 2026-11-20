import "reflect-metadata";
import { DataSource, getMetadataArgsStorage } from "typeorm";
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
import { GrantFeedback } from "../entities/GrantFeedback";
import { UsedNonce } from "../entities/UsedNonce";
import { Dispute } from "../entities/Dispute";
import { MilestoneAppeal } from "../entities/MilestoneAppeal";

const entities = [
  Activity,
  AuditLog,
  Community,
  Contributor,
  Dispute,
  FeeCollection,
  Grant,
  GrantFeedback,
  GrantHistory,
  GrantReviewer,
  GrantView,
  Milestone,
  MilestoneAppeal,
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
  UsedNonce,
  WebhookDeliveryLog,
  WebhookSubscription,
];

// The entities Dispute, MilestoneAppeal, and UsedNonce declare date columns
// with an explicit type string via timestampColumnType. Because TypeORM
// decorator metadata is evaluated at class-definition time (before any
// DataSource is created), the type is fixed to whichever value column-types.ts
// returned first. We patch the global metadata storage here so the correct
// type is in place before the DataSource validates entity metadata.
const DATE_COLUMN_PATCHES: Array<{ target: Function; propertyName: string }> = [
  { target: UsedNonce, propertyName: "expiresAt" },
  { target: Dispute, propertyName: "resolvedAt" },
  { target: MilestoneAppeal, propertyName: "resolvedAt" },
];

function patchEntityDateColumns(type: "datetime" | "timestamp"): void {
  const storage = getMetadataArgsStorage();
  for (const patch of DATE_COLUMN_PATCHES) {
    const col = storage.columns.find(
      (c) => c.target === patch.target && c.propertyName === patch.propertyName,
    );
    if (col && col.options) {
      col.options.type = type;
    }
  }
}

export const shouldSynchronize = (nodeEnv: string = env.nodeEnv) =>
  nodeEnv !== "production";

export const buildDataSource = (databaseUrl = env.databaseUrl) => {
  const isSqljs = databaseUrl.startsWith("sqljs");
  // Patch timestamp columns that can't use TypeORM type inference at decoration
  // time — sqljs requires "datetime", postgres requires "timestamp".
  const dateType = isSqljs ? "datetime" : "timestamp";
  patchEntityDateColumns(dateType);

  return new DataSource({
    type: isSqljs ? "sqljs" : "postgres",
    ...(isSqljs
      ? { location: databaseUrl.replace("sqljs://", ""), autoSave: false }
      : { url: databaseUrl }),
    entities,
    // Never let TypeORM auto-alter a production schema; use migrations there.
    synchronize: shouldSynchronize(),
  });
};
