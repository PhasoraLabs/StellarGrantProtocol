import { Column, CreateDateColumn, Entity, Index, PrimaryGeneratedColumn } from "typeorm";

@Entity({ name: "notifications" })
@Index("IDX_notifications_address_read", ["address", "readAt"])
export class Notification {
  @PrimaryGeneratedColumn()
  id!: number;

  @Column({ type: "varchar", length: 120 })
  address!: string;

  @Column({ type: "varchar", length: 80 })
  type!: string;

  @Column({ type: "simple-json" })
  payload!: Record<string, unknown>;

  @Column({ type: "timestamp", nullable: true })
  readAt!: Date | null;

  @CreateDateColumn()
  createdAt!: Date;
}
