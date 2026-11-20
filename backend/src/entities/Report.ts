import { Column, CreateDateColumn, Entity, JoinColumn, ManyToOne, PrimaryGeneratedColumn } from "typeorm";
import { Grant } from "./Grant";

@Entity({ name: "reports" })
export class Report {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Column({ type: "varchar", length: 120 })
  reporterAddress!: string;

  @Column({ type: "int", nullable: true })
  grantId!: number | null;

  @Column({ type: "text" })
  reason!: string;

  @Column({ type: "varchar", length: 30, default: "pending" })
  status!: string;

  @CreateDateColumn()
  createdAt!: Date;

  @ManyToOne(() => Grant, { onDelete: "SET NULL", nullable: true })
  @JoinColumn({ name: "grantId" })
  grant!: Grant | null;
}
