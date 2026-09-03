import {
  Column,
  Entity,
  JoinColumn,
  ManyToOne,
  PrimaryGeneratedColumn,
  Unique,
  Index,
} from "typeorm";
import { Grant } from "./Grant";

@Entity({ name: "milestones" })
@Unique(["grantId", "idx"])
export class Milestone {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Index()
  @Column({ type: "int" })
  grantId!: number;

  @Column({ type: "int" })
  idx!: number;

  @Column({ type: "varchar", length: 200 })
  title!: string;

  @Column({ type: "text", nullable: true })
  description!: string | null;

  @Column({ type: "varchar", length: 40 })
  deadline!: string;

  @Column({ type: "varchar", length: 40, nullable: true })
  lastDeadlineReminderAt!: string | null;

  @Column({ type: "int", nullable: true })
  lastDeadlineReminderDaysBefore!: number | null;

  @Column({ type: "varchar", length: 40, nullable: true })
  overdueNotifiedAt!: string | null;

  @ManyToOne(() => Grant, (grant) => grant.milestones, { onDelete: "CASCADE" })
  @JoinColumn({ name: "grantId" })
  grant!: Grant;
}
