import { Column, CreateDateColumn, Entity, Index, PrimaryGeneratedColumn } from "typeorm";

@Entity({ name: "reputation_logs" })
export class ReputationLog {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Column({ type: "varchar", length: 120 })
  @Index("IDX_reputation_logs_address")
  address!: string;

  @Column({ type: "int" })
  gain!: number;

  @CreateDateColumn()
  @Index("IDX_reputation_logs_timestamp")
  timestamp!: Date;
}
