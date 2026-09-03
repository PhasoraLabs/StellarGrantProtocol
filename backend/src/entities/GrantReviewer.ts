import { Entity, PrimaryGeneratedColumn, Column, ManyToOne, JoinColumn } from "typeorm";
import { Grant } from "./Grant";

@Entity({ name: "grant_reviewers" })
export class GrantReviewer {
  @PrimaryGeneratedColumn()
  id!: number;

  @Column({ type: "int" })
  grantId!: number;

  @Column({ type: "varchar", length: 56 })
  reviewerStellarAddress!: string;

  @ManyToOne(() => Grant, (grant) => grant.reviewers, { onDelete: "CASCADE" })
  @JoinColumn({ name: "grantId" })
  grant!: Grant;
}
