import { Column, CreateDateColumn, Entity, PrimaryGeneratedColumn } from "typeorm";

@Entity({ name: "fee_collections" })
export class FeeCollection {
  @PrimaryGeneratedColumn()
  id!: number;

  @Column({ type: "int" })
  grantId!: number;

  @Column({ type: "varchar", length: 120 })
  funderAddress!: string;

  @Column({ type: "varchar", length: 30 })
  token!: string;

  @Column({ type: "varchar", length: 60 })
  totalContribution!: string;

  @Column({ type: "varchar", length: 60 })
  feeAmount!: string;

  @Column({ type: "varchar", length: 30 })
  feePercentage!: string;

  @CreateDateColumn()
  createdAt!: Date;
}
