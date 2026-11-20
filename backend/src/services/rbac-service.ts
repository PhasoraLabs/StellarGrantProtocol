import { Repository } from "typeorm";
import { User } from "../entities/User";
import { Role, RoleName } from "../entities/Role";
import { UserRole } from "../entities/UserRole";
import { getRolePermissions, Permission, roleHasPermission } from "../config/rbac";

export class RbacService {
  constructor(
    private userRepo: Repository<User>,
    private roleRepo: Repository<Role>,
    private userRoleRepo: Repository<UserRole>,
  ) {}

  /**
   * Get all roles for a user
   */
  async getUserRoles(userId: number): Promise<Role[]> {
    const userRoles = await this.userRoleRepo.find({
      where: { userId },
      relations: ["role"],
    });
    return userRoles.map((ur) => ur.role);
  }

  /**
   * Get all roles for a user by stellar address
   */
  async getUserRolesByAddress(stellarAddress: string): Promise<Role[]> {
    const user = await this.userRepo.findOne({ where: { stellarAddress } });
    if (!user) {
      return [];
    }
    return this.getUserRoles(user.id);
  }

  /**
   * Get all permissions for a user
   */
  async getUserPermissions(userId: number): Promise<Permission[]> {
    const roles = await this.getUserRoles(userId);
    const permissions = new Set<Permission>();

    for (const role of roles) {
      const rolePerms = getRolePermissions(role.name);
      rolePerms.forEach((perm) => permissions.add(perm));
    }

    return Array.from(permissions);
  }

  /**
   * Get all permissions for a user by stellar address
   */
  async getUserPermissionsByAddress(stellarAddress: string): Promise<Permission[]> {
    const user = await this.userRepo.findOne({ where: { stellarAddress } });
    if (!user) {
      return [];
    }
    return this.getUserPermissions(user.id);
  }

  /**
   * Check if a user has a specific permission
   */
  async userHasPermission(userId: number, permission: Permission): Promise<boolean> {
    const roles = await this.getUserRoles(userId);
    return roles.some((role) => roleHasPermission(role.name, permission));
  }

  /**
   * Check if a user has a specific permission by stellar address
   */
  async userHasPermissionByAddress(stellarAddress: string, permission: Permission): Promise<boolean> {
    const roles = await this.getUserRolesByAddress(stellarAddress);
    return roles.some((role) => roleHasPermission(role.name, permission));
  }

  /**
   * Check if a user has any of the specified permissions
   */
  async userHasAnyPermission(userId: number, permissions: Permission[]): Promise<boolean> {
    const userPerms = await this.getUserPermissions(userId);
    return permissions.some((perm) => userPerms.includes(perm));
  }

  /**
   * Check if a user has any of the specified permissions by stellar address
   */
  async userHasAnyPermissionByAddress(stellarAddress: string, permissions: Permission[]): Promise<boolean> {
    const userPerms = await this.getUserPermissionsByAddress(stellarAddress);
    return permissions.some((perm) => userPerms.includes(perm));
  }

  /**
   * Assign a role to a user
   */
  async assignRole(userId: number, roleName: RoleName, assignedBy: string): Promise<UserRole> {
    const user = await this.userRepo.findOne({ where: { id: userId } });
    if (!user) {
      throw new Error("User not found");
    }

    const role = await this.roleRepo.findOne({ where: { name: roleName } });
    if (!role) {
      throw new Error("Role not found");
    }

    // Check if user already has this role
    const existing = await this.userRoleRepo.findOne({
      where: { userId, roleId: role.id },
    });
    if (existing) {
      throw new Error("User already has this role");
    }

    const userRole = this.userRoleRepo.create({
      userId,
      roleId: role.id,
      assignedBy,
    });

    return this.userRoleRepo.save(userRole);
  }

  /**
   * Assign a role to a user by stellar address
   */
  async assignRoleByAddress(stellarAddress: string, roleName: RoleName, assignedBy: string): Promise<UserRole> {
    const user = await this.userRepo.findOne({ where: { stellarAddress } });
    if (!user) {
      throw new Error("User not found");
    }
    return this.assignRole(user.id, roleName, assignedBy);
  }

  /**
   * Revoke a role from a user
   */
  async revokeRole(userId: number, roleName: RoleName): Promise<void> {
    const role = await this.roleRepo.findOne({ where: { name: roleName } });
    if (!role) {
      throw new Error("Role not found");
    }

    await this.userRoleRepo.delete({
      userId,
      roleId: role.id,
    });
  }

  /**
   * Revoke a role from a user by stellar address
   */
  async revokeRoleByAddress(stellarAddress: string, roleName: RoleName): Promise<void> {
    const user = await this.userRepo.findOne({ where: { stellarAddress } });
    if (!user) {
      throw new Error("User not found");
    }
    return this.revokeRole(user.id, roleName);
  }

  /**
   * Initialize default roles in the database
   */
  async initializeDefaultRoles(): Promise<void> {
    const { ROLE_PERMISSIONS } = await import("../config/rbac");

    for (const [roleName, permissions] of Object.entries(ROLE_PERMISSIONS)) {
      let role = await this.roleRepo.findOne({ where: { name: roleName as RoleName } });
      if (!role) {
        role = this.roleRepo.create({
          name: roleName as RoleName,
          permissions,
          description: this.getRoleDescription(roleName as RoleName),
        });
        await this.roleRepo.save(role);
      } else {
        // Update permissions if they differ
        role.permissions = permissions;
        await this.roleRepo.save(role);
      }
    }
  }

  /**
   * Get role description
   */
  private getRoleDescription(role: RoleName): string {
    const descriptions: Record<RoleName, string> = {
      [RoleName.USER]: "Standard user with basic permissions",
      [RoleName.GRANTEE]: "Grant recipient with ability to create and manage grants",
      [RoleName.REVIEWER]: "Grant reviewer with ability to approve/reject milestones",
      [RoleName.ADMIN]: "Administrator with full system access",
    };
    return descriptions[role];
  }
}
