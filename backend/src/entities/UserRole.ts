import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, ManyToOne, JoinColumn, Unique } from "typeorm";
import { User } from "./User";
import { Role } from "./Role";

@Entity({ name: "user_roles" })
@Unique(["userId", "roleId"])
export class UserRole {
  @PrimaryGeneratedColumn()
  id!: number;

  @Column({ name: "user_id", type: "int" })
  userId!: number;

  @Column({ name: "role_id", type: "int" })
  roleId!: number;

  @Column({ name: "assigned_by", type: "varchar", length: 120, nullable: true })
  assignedBy!: string | null;

  @ManyToOne(() => User, { onDelete: "CASCADE" })
  @JoinColumn({ name: "user_id" })
  user!: User;

  @ManyToOne(() => Role, { onDelete: "CASCADE" })
  @JoinColumn({ name: "role_id" })
  role!: Role;

  @CreateDateColumn({ name: "created_at" })
  createdAt!: Date;
}
