import { Column, CreateDateColumn, Entity, Index, PrimaryGeneratedColumn, Unique } from "typeorm";

@Entity({ name: "milestone_appeals" })
@Unique(["grantId", "milestoneIdx"])
export class MilestoneAppeal {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Index()
  @Column({ type: "int" })
  grantId!: number;

  @Column({ type: "int" })
  milestoneIdx!: number;

  @Column({ type: "text" })
  reason!: string;

  @Column({ type: "varchar", length: 30, default: "pending" })
  status!: "pending" | "upheld" | "denied";

  @Column({ type: "simple-json", default: "[]" })
  reviewerVotes!: Array<{ reviewer: string; uphold: boolean; votedAt: string }>;

  @Column({ type: "varchar", length: 120 })
  appellantAddress!: string;

  @CreateDateColumn()
  openedAt!: Date;

  @Column({ type: "timestamp", nullable: true })
  resolvedAt!: Date | null;
}
