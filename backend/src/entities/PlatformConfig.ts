import { Column, Entity, PrimaryColumn, UpdateDateColumn } from "typeorm";

@Entity({ name: "platform_configs" })
export class PlatformConfig {
  @PrimaryColumn({ type: "varchar", length: 50 })
  key!: string;

  @Column({ type: "varchar", length: 255 })
  value!: string;

  @UpdateDateColumn()
  updatedAt!: Date;
}
