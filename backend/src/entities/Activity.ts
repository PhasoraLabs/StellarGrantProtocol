import { Column, CreateDateColumn, Entity, Index, PrimaryGeneratedColumn } from "typeorm";

export type ActivityType = 
  | "grant_created"
  | "grant_updated"
  | "grant_funded"
  | "grant_completed"
  | "milestone_submitted"
  | "milestone_approved"
  | "milestone_reactivated"
  | "reputation_gained"
  | "watchlist_added"
  | "watchlist_removed";

export type EntityType = "grant" | "contributor" | "milestone_proof";

@Entity({ name: "activities" })
@Index("IDX_activities_timestamp", ["timestamp"])
@Index("IDX_activities_type", ["type"])
@Index("IDX_activities_entity", ["entityType", "entityId"])
export class Activity {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Column({ type: "varchar", length: 50 })
  type!: ActivityType;

  @Column({ type: "varchar", length: 50 })
  entityType!: EntityType;

  @Column({ type: "int", nullable: true })
  entityId!: number | null;

  @Column({ type: "varchar", length: 120, nullable: true })
  actorAddress!: string | null;

  @Column({ type: "simple-json", nullable: true })
  data!: Record<string, unknown> | null;

  @CreateDateColumn()
  timestamp!: Date;
}
