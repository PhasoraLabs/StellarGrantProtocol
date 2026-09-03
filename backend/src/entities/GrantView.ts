import { Column, CreateDateColumn, Entity, Index, PrimaryGeneratedColumn } from "typeorm";

@Entity({ name: "grant_views" })
@Index("IDX_grant_views_grant_id", ["grantId"])
@Index("IDX_grant_views_created_at", ["createdAt"])
@Index("IDX_grant_views_dedup", ["grantId", "viewerKey", "hourBucket"], { unique: true })
export class GrantView {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Column({ type: "int" })
  grantId!: number;

  /** Hashed viewer identifier (address or IP). Stored as a hash to avoid storing PII. */
  @Column({ type: "varchar", length: 64 })
  viewerKey!: string;

  /** Truncated to the hour for deduplication: one view per viewer per hour per grant. */
  @Column({ type: "varchar", length: 13 })
  hourBucket!: string;

  @CreateDateColumn()
  createdAt!: Date;
}
