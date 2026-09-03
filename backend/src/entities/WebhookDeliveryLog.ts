import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, UpdateDateColumn, ManyToOne, JoinColumn } from "typeorm";
import { WebhookSubscription } from "./WebhookSubscription";

/**
 * Status of a webhook delivery attempt
 */
export enum WebhookDeliveryStatus {
  PENDING = "pending",
  DELIVERED = "delivered",
  FAILED = "failed",
  RETRYING = "retrying",
  EXHAUSTED = "exhausted",
}

@Entity({ name: "webhook_delivery_logs" })
export class WebhookDeliveryLog {
  @PrimaryGeneratedColumn()
  id!: number;

  @Column({ name: "subscription_id", type: "int" })
  subscriptionId!: number;

  @ManyToOne(() => WebhookSubscription, { onDelete: "CASCADE" })
  @JoinColumn({ name: "subscription_id" })
  subscription!: WebhookSubscription;

  @Column({ name: "event_type", type: "varchar", length: 64 })
  eventType!: string;

  @Column({ name: "payload", type: "simple-json" })
  payload!: Record<string, any>;

  @Column({ name: "payload_signature", type: "varchar", length: 128 })
  payloadSignature!: string;

  @Column({ type: "varchar", length: 20, default: WebhookDeliveryStatus.PENDING, name: "status" })
  status!: WebhookDeliveryStatus;

  @Column({ type: "int", default: 0, name: "attempt_count" })
  attemptCount!: number;

  @Column({ type: "int", nullable: true, name: "http_status_code" })
  httpStatusCode!: number | null;

  @Column({ type: "text", nullable: true, name: "response_body" })
  responseBody!: string | null;

  @Column({ type: "text", nullable: true, name: "error_message" })
  errorMessage!: string | null;

  @Column({ type: "varchar", length: 100, nullable: true, name: "next_retry_at" })
  nextRetryAt!: string | null;

  @CreateDateColumn({ name: "created_at" })
  createdAt!: Date;

  @UpdateDateColumn({ name: "updated_at" })
  updatedAt!: Date;
}
