import {
  Column,
  Entity,
  Index,
  JoinColumn,
  ManyToOne,
  OneToMany,
  PrimaryGeneratedColumn,
  UpdateDateColumn,
  ManyToMany,
  JoinTable,
} from "typeorm";
import { MilestoneProof } from "./MilestoneProof";
import { Milestone } from "./Milestone";
import { Community } from "./Community";
import { GrantReviewer } from "./GrantReviewer";

@Entity({ name: "grants" })
@Index("IDX_grant_community_id", ["communityId"])
@Index("IDX_grant_updated_at_id", ["updatedAt", "id"])
export class Grant {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Column({ type: "varchar", length: 200 })
  title!: string;

  @Column({ type: "varchar", length: 500, nullable: true })
  description?: string;

  @Column({ type: "varchar", length: 30 })
  status!: string;

  @Column({ type: "varchar", length: 120 })
  owner!: string;

  @Column({ type: "varchar", length: 120 })
  recipient!: string;

  @Column({ type: "int", nullable: true })
  communityId?: number | null;

  @ManyToOne(() => Community, (community) => community.grants, {
    nullable: true,
  })
  @JoinColumn({ name: "communityId" })
  community?: Community;

  @Column({ type: "varchar", length: 60 })
  totalAmount!: string;

  /** Funding token sentinel ("native") or asset/SAC identifier (e.g. "USDC"). */
  @Column({ type: "varchar", length: 120, nullable: true, default: "native" })
  token?: string | null;

  @Column({ type: "text", nullable: true })
  tags?: string;

  @Column({ type: "boolean", default: false })
  isDraft!: boolean;

  @Column({ type: "simple-json", nullable: true })
  draftData!: Record<string, unknown> | null;

  @OneToMany(() => GrantReviewer, (reviewer) => reviewer.grant)
  reviewers?: GrantReviewer[];

  @OneToMany(() => Milestone, (milestone) => milestone.grant)
  milestones?: Milestone[];

  @UpdateDateColumn()
  updatedAt!: Date;

  @OneToMany(() => MilestoneProof, (proof) => proof.grant)
  proofs!: MilestoneProof[];
}
