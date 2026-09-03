import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, UpdateDateColumn, OneToMany } from "typeorm";
import { UserRole } from "./UserRole";

export enum RoleName {
  USER = "user",
  GRANTEE = "grantee",
  REVIEWER = "reviewer",
  ADMIN = "admin",
}

@Entity({ name: "roles" })
export class Role {
  @PrimaryGeneratedColumn()
  id!: number;

  @Column({ type: "varchar", length: 50, unique: true })
  name!: RoleName;

  @Column({ type: "text", nullable: true })
  description!: string | null;

  @Column({ type: "simple-array" })
  permissions!: string[];

  @CreateDateColumn()
  createdAt!: Date;

  @UpdateDateColumn()
  updatedAt!: Date;

  @OneToMany(() => UserRole, (userRole) => userRole.role)
  userRoles!: UserRole[];
}
