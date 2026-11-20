import {
  Entity,
  PrimaryGeneratedColumn,
  Column,
  CreateDateColumn,
  Index,
} from "typeorm";

@Entity({ name: "rate_limit_logs" })
export class RateLimitLog {
  @PrimaryGeneratedColumn()
  id!: number;

  @Column({ type: "varchar", length: 64 })
  ip!: string;

  @Column({ type: "varchar", length: 255 })
  path!: string;

  @Column({ type: "varchar", length: 16 })
  method!: string;

  @Column({ type: "varchar", length: 255, nullable: true })
  userAgent!: string;

  @Column({ type: "varchar", length: 120, nullable: true })
  address!: string | null;

  
  @Index("IDX_rate_limit_created_at", ["createdAt"])
  @CreateDateColumn()
  createdAt!: Date;

}
