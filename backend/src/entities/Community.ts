import { Column, CreateDateColumn, Entity, OneToMany, PrimaryGeneratedColumn, UpdateDateColumn } from "typeorm";
import { Grant } from "./Grant";

@Entity({ name: "communities" })
export class Community {
  @PrimaryGeneratedColumn("increment")
  id!: number;

  @Column({ type: "varchar", length: 120, unique: true })
  name!: string;

  @Column({ type: "text", nullable: true })
  description!: string | null;

  @Column({ type: "text", nullable: true })
  logoUrl!: string | null;

  @Column({ type: "simple-array", nullable: true })
  adminAddresses!: string[] | null;

  @Column({ type: "boolean", default: false })
  featured!: boolean;

  @CreateDateColumn()
  createdAt!: Date;

  @UpdateDateColumn()
  updatedAt!: Date;

  @OneToMany(() => Grant, (grant) => grant.community)
  grants!: Grant[];
}
