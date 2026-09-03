import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, UpdateDateColumn, ManyToOne, JoinColumn } from "typeorm";
import { User } from "./User";

/**
 * Supported webhook event types
 */
export enum WebhookEventType {
  GRANT_CREATED = "grant.created",
  GRANT_UPDATED = "grant.updated",
  GRANT_STATUS_CHANGED = "grant.status_changed",
  MILESTONE_SUBMITTED = "milestone.submitted",
  MILESTONE_APPROVED = "milestone.approved",
  MILESTONE_REJECTED = "milestone.rejected",
  CONTRIBUTOR_BLACKLISTED = "contributor.blacklisted",
  CONTRIBUTOR_REPUTATION_CHANGED = "contributor.reputation_changed",
  FEE_COLLECTED = "fee.collected",
  COMMUNITY_CREATED = "community.created",
  COMMUNITY_UPDATED = "community.updated",
  WATCHLIST_ADDED = "watchlist.added",
  WATCHLIST_REMOVED = "watchlist.removed",
  ALL = "*",
}

@Entity({ name: "webhook_subscriptions" })
export class WebhookSubscription {
  @PrimaryGeneratedColumn()
  id!: number;

  @Column({ name: "target_url", type: "varchar", length: 2048 })
  targetUrl!: string;

  @Column({ name: "secret_key", type: "varchar", length: 255 })
  secretKey!: string;

  @Column({ type: "simple-array" })
  events!: WebhookEventType[];

  @Column({ type: "boolean", default: true, name: "is_active" })
  isActive!: boolean;

  @Column({ type: "int", default: 0, name: "failure_count" })
  failureCount!: number;

  @Column({ type: "int", default: 5, name: "max_retries" })
  maxRetries!: number;

  @Column({ name: "community_id", type: "int", nullable: true })
  communityId!: number | null;

  @Column({ name: "owner_address", type: "varchar", length: 56, nullable: true })
  ownerAddress!: string | null;

  @Column({ name: "created_by", type: "int" })
  createdBy!: number;

  @ManyToOne(() => User, { onDelete: "CASCADE" })
  @JoinColumn({ name: "created_by" })
  user!: User;

  @CreateDateColumn({ name: "created_at" })
  createdAt!: Date;

  @UpdateDateColumn({ name: "updated_at" })
  updatedAt!: Date;
}
