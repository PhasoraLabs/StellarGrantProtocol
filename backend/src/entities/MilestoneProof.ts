import {
  Column,
  CreateDateColumn,
  Entity,
  JoinColumn,
  ManyToOne,
  PrimaryGeneratedColumn,
  Unique,
} from "typeorm";
import { Grant } from "./Grant";

@Entity({ name: "milestone_proofs" })
@Unique(["grantId", "milestoneIdx"])
export class MilestoneProof {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Column({ type: "int" })
  grantId!: number;

  @Column({ type: "int" })
  milestoneIdx!: number;

  @Column({ type: "varchar", length: 255 })
  proofCid!: string;

  @Column({ type: "text", nullable: true })
  description?: string | null;

  @Column({ type: "varchar", length: 120 })
  submittedBy!: string;

  @Column({ type: "varchar", length: 255 })
  signature!: string;

  @Column({ type: "varchar", length: 80 })
  nonce!: string;

  @CreateDateColumn()
  createdAt!: Date;

  @ManyToOne(() => Grant, (grant) => grant.proofs, { onDelete: "CASCADE" })
  @JoinColumn({ name: "grantId" })
  grant!: Grant;
}
