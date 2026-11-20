import { Column, Entity, OneToMany, PrimaryColumn, UpdateDateColumn } from "typeorm";
import { MilestoneProof } from "./MilestoneProof";

@Entity({ name: "grants" })
export class Grant {
  @PrimaryColumn({ type: "int" })
  id!: number;

  @Column({ type: "varchar", length: 200 })
  title!: string;

  @Column({ type: "text", default: "" })
  description!: string;

  @Column({ type: "varchar", length: 30 })
  status!: string;

  @Column({ type: "varchar", length: 120 })
  recipient!: string;

  @Column({ type: "varchar", length: 60 })
  totalAmount!: string;

  @UpdateDateColumn()
  updatedAt!: Date;

  @OneToMany(() => MilestoneProof, (proof) => proof.grant)
  proofs!: MilestoneProof[];
}
