import {
  Column,
  CreateDateColumn,
  Entity,
  Index,
  PrimaryGeneratedColumn,
  Unique,
} from "typeorm";
import { timestampColumnType } from "../db/column-types";

@Entity({ name: "disputes" })
@Unique(["grantId", "milestoneIdx"])
export class Dispute {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Index()
  @Column({ type: "int" })
  grantId!: number;

  @Column({ type: "int" })
  milestoneIdx!: number;

  @Column({ type: "varchar", length: 30, default: "open" })
  status!: "open" | "resolved_payout" | "resolved_refund";

  @Column({ type: "text", nullable: true })
  contributorArgument!: string | null;

  @Column({ type: "text", nullable: true })
  funderArgument!: string | null;

  @Column({ type: "varchar", length: 120, nullable: true })
  resolvedBy!: string | null;

  @Column({ type: "varchar", length: 64, nullable: true })
  resolutionTxHash!: string | null;

  @CreateDateColumn()
  openedAt!: Date;

  @Column({ type: timestampColumnType, nullable: true })
  resolvedAt!: Date | null;
}
