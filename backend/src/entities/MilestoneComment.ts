import {
  Column,
  CreateDateColumn,
  Entity,
  JoinColumn,
  ManyToOne,
  PrimaryGeneratedColumn,
} from "typeorm";
import { Milestone } from "./Milestone";

@Entity({ name: "milestone_comments" })
export class MilestoneComment {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Column({ type: "int" })
  milestoneId!: number;

  @Column({ type: "text" })
  content!: string;

  @Column({ type: "varchar", length: 120 })
  authorAddress!: string;

  @Column({ type: "int", nullable: true })
  parentCommentId!: number | null;

  @CreateDateColumn()
  createdAt!: Date;

  @ManyToOne(() => Milestone, { onDelete: "CASCADE" })
  @JoinColumn({ name: "milestoneId" })
  milestone!: Milestone;
}
