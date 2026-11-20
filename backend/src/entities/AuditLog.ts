import { Column, CreateDateColumn, Entity, PrimaryGeneratedColumn } from "typeorm";

@Entity({ name: "audit_logs" })
export class AuditLog {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Column({ type: "varchar", length: 120 })
  adminAddress!: string;

  @Column({ type: "varchar", length: 50 })
  action!: string;

  @Column({ type: "text", nullable: true })
  target!: string | null;

  @Column({ type: "text", nullable: true })
  details!: string | null;

  @CreateDateColumn()
  createdAt!: Date;
}
