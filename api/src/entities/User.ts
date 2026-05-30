import { Column, Entity, PrimaryGeneratedColumn } from "typeorm";

@Entity()
export class User {
  @PrimaryGeneratedColumn()
  id!: number;

  @Column({ type: "varchar", length: 56, unique: true })
  stellarAddress!: string;

  @Column({ type: "varchar", nullable: true })
  email?: string;

  @Column({ type: "boolean", default: false })
  notifyMilestoneApproved = false;

  @Column({ type: "boolean", default: false })
  notifyMilestoneSubmitted = false;

  @Column({ type: "varchar", nullable: true })
  githubId?: string;

  @Column({ type: "varchar", nullable: true })
  githubUsername?: string;

  @Column({ type: "varchar", nullable: true })
  twitterId?: string;

  @Column({ type: "varchar", nullable: true })
  twitterUsername?: string;
}
