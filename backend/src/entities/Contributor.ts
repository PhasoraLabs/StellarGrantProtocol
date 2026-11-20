import { Column, Entity, Index, PrimaryColumn, UpdateDateColumn } from "typeorm";

@Entity({ name: "contributors" })
export class Contributor {
  @PrimaryColumn({ type: "varchar", length: 120 })
  address!: string;

  @Column({ type: "int", default: 0 })
  @Index("IDX_contributors_reputation")
  reputation!: number;

  @Column({ type: "int", default: 0 })
  totalGrantsCompleted!: number;

  @Column({ type: "boolean", default: false })
  isBlacklisted!: boolean;

  @Column({ type: "varchar", length: 254, nullable: true })
  email!: string | null;

  @Column({ type: "text", nullable: true })
  bio!: string | null;

  @Column({ type: "varchar", length: 2048, nullable: true })
  profilePictureUrl!: string | null;

  @Column({ type: "varchar", length: 2048, nullable: true })
  githubUrl!: string | null;

  @Column({ type: "varchar", length: 2048, nullable: true })
  twitterUrl!: string | null;

  @Column({ type: "varchar", length: 2048, nullable: true })
  linkedinUrl!: string | null;

  @Column({ type: "boolean", default: true })
  emailNotifications!: boolean;

  @UpdateDateColumn()
  updatedAt!: Date;
}
