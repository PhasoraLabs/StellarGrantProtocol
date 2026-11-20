import { Column, CreateDateColumn, Entity, JoinColumn, ManyToOne, PrimaryGeneratedColumn } from "typeorm";
import { Grant } from "./Grant";

@Entity({ name: "grant_history" })
export class GrantHistory {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Column({ type: "int", nullable: true })
  grantId!: number | null;

  @Column({ type: "simple-json" })
  snapshot!: any;

  @Column({ type: "simple-json" })
  diff!: any;

  @CreateDateColumn()
  createdAt!: Date;

  @ManyToOne(() => Grant, { onDelete: "SET NULL", nullable: true })
  @JoinColumn({ name: "grantId" })
  grant!: Grant | null;
}
