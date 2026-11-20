import { RoleName } from "../entities/Role";

/**
 * Permission definitions for the RBAC system
 * Format: resource:action
 */
export const PERMISSIONS = {
  // Grant permissions
  "grants:read": "View grants",
  "grants:create": "Create new grants",
  "grants:update": "Update grants",
  "grants:delete": "Delete grants",
  "grants:report": "Report grants",
  "grants:manage": "Full grant management",

  // Milestone permissions
  "milestones:read": "View milestones",
  "milestones:create": "Submit milestone proofs",
  "milestones:approve": "Approve milestones",
  "milestones:reject": "Reject milestones",
  "milestones:comment": "Comment on milestones",

  // Profile permissions
  "profiles:read": "View profiles",
  "profiles:update_own": "Update own profile",
  "profiles:update_any": "Update any profile",

  // User permissions
  "users:read": "View users",
  "users:update": "Update users",
  "users:assign_roles": "Assign roles to users",
  "users:blacklist": "Blacklist users",

  // Community permissions
  "communities:read": "View communities",
  "communities:create": "Create communities",
  "communities:update": "Update communities",
  "communities:delete": "Delete communities",
  "communities:manage": "Full community management",

  // Admin permissions
  "admin:sync": "Sync grants from blockchain",
  "admin:reconcile": "Run reconciliation",
  "admin:bulk_actions": "Perform bulk actions on grants",
  "admin:config": "Manage platform configuration",
  "admin:metrics": "View metrics and rate limits",
  "admin:all": "Full admin access",

  // Watchlist permissions
  "watchlist:manage": "Manage watchlist",

  // Analytics permissions
  "analytics:read": "View analytics",
} as const;

export type Permission = keyof typeof PERMISSIONS;

/**
 * Default role permissions mapping
 */
export const ROLE_PERMISSIONS: Record<RoleName, Permission[]> = {
  [RoleName.USER]: [
    "grants:read",
    "milestones:read",
    "profiles:read",
    "profiles:update_own",
    "watchlist:manage",
  ],

  [RoleName.GRANTEE]: [
    "grants:read",
    "grants:create",
    "grants:update",
    "milestones:read",
    "milestones:create",
    "milestones:comment",
    "profiles:read",
    "profiles:update_own",
    "watchlist:manage",
  ],

  [RoleName.REVIEWER]: [
    "grants:read",
    "grants:report",
    "milestones:read",
    "milestones:approve",
    "milestones:reject",
    "milestones:comment",
    "profiles:read",
    "profiles:update_own",
    "analytics:read",
  ],

  [RoleName.ADMIN]: [
    "admin:all",
  ],
};

/**
 * Get all permissions for a role
 */
export function getRolePermissions(role: RoleName): Permission[] {
  return ROLE_PERMISSIONS[role] || [];
}

/**
 * Check if a role has a specific permission
 */
export function roleHasPermission(role: RoleName, permission: Permission): boolean {
  // Admin has all permissions
  if (role === RoleName.ADMIN) {
    return true;
  }

  const permissions = getRolePermissions(role);
  return permissions.includes(permission);
}

/**
 * Check if a role has any of the specified permissions
 */
export function roleHasAnyPermission(role: RoleName, permissions: Permission[]): boolean {
  // Admin has all permissions
  if (role === RoleName.ADMIN) {
    return true;
  }

  const rolePermissions = getRolePermissions(role);
  return permissions.some((perm) => rolePermissions.includes(perm));
}
