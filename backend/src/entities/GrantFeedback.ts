import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, Unique, Index } from "typeorm";

@Entity({ name: "grant_feedback" })
@Unique(["grantId", "reviewerAddress"])
export class GrantFeedback {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Index()
  @Column({ type: "int" })
  grantId!: number;

  @Column({ type: "varchar", length: 120 })
  reviewerAddress!: string;

  @Column({ type: "varchar", length: 20 })
  role!: "funder" | "reviewer" | "recipient";

  @Column({ type: "smallint" })
  rating!: number; // 1-5

  @Column({ type: "text", nullable: true })
  comment!: string | null;

  @CreateDateColumn()
  createdAt!: Date;
}
