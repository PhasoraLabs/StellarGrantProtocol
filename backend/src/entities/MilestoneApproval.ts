import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn } from "typeorm";

@Entity({ name: "milestone_approvals" })
export class MilestoneApproval {
  @PrimaryGeneratedColumn()
  id!: number;

  @Column({ type: "int" })
  grantId!: number;

  @Column({ type: "int" })
  milestoneIdx!: number;

  @Column({ type: "varchar", length: 56 })
  reviewerStellarAddress!: string;

  @Column({ type: "boolean" })
  approved!: boolean;

  @CreateDateColumn()
  createdAt!: Date;
}
