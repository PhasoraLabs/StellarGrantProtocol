# Role-Based Access Control (RBAC) System

This document describes the RBAC system implemented for the StellarGrant API.

## Overview

The RBAC system provides structured access control across the API with four core roles:
- **User**: Basic permissions for viewing grants and updating own profile
- **Grantee**: Can create and manage grants, submit milestone proofs
- **Reviewer**: Can review and approve/reject milestones
- **Admin**: Full system access

## Database Schema

### Role Entity
Located in `src/entities/Role.ts`
- `id`: Primary key
- `name`: Role name (enum: user, grantee, reviewer, admin)
- `description`: Role description
- `permissions`: JSON array of permissions

### UserRole Entity
Located in `src/entities/UserRole.ts`
- `userId`: Foreign key to User
- `roleId`: Foreign key to Role
- `assignedBy`: Stellar address of who assigned the role
- `createdAt`: Assignment timestamp

### User Entity
Updated in `src/entities/User.ts`
- Added `userRoles` relationship to UserRole entity

## Permissions

Permissions are defined in `src/config/rbac.ts` with the format `resource:action`:

### Grant Permissions
- `grants:read`: View grants
- `grants:create`: Create new grants
- `grants:update`: Update grants
- `grants:delete`: Delete grants
- `grants:report`: Report grants
- `grants:manage`: Full grant management

### Milestone Permissions
- `milestones:read`: View milestones
- `milestones:create`: Submit milestone proofs
- `milestones:approve`: Approve milestones
- `milestones:reject`: Reject milestones
- `milestones:comment`: Comment on milestones

### Profile Permissions
- `profiles:read`: View profiles
- `profiles:update_own`: Update own profile
- `profiles:update_any`: Update any profile

### User Permissions
- `users:read`: View users
- `users:update`: Update users
- `users:assign_roles`: Assign roles to users
- `users:blacklist`: Blacklist users

### Community Permissions
- `communities:read`: View communities
- `communities:create`: Create communities
- `communities:update`: Update communities
- `communities:delete`: Delete communities
- `communities:manage`: Full community management

### Admin Permissions
- `admin:sync`: Sync grants from blockchain
- `admin:reconcile`: Run reconciliation
- `admin:bulk_actions`: Perform bulk actions on grants
- `admin:config`: Manage platform configuration
- `admin:metrics`: View metrics and rate limits
- `admin:all`: Full admin access

### Other Permissions
- `watchlist:manage`: Manage watchlist
- `analytics:read`: View analytics

## Role Permissions Mapping

### User Role
- grants:read
- milestones:read
- profiles:read
- profiles:update_own
- watchlist:manage

### Grantee Role
- grants:read
- grants:create
- grants:update
- milestones:read
- milestones:create
- milestones:comment
- profiles:read
- profiles:update_own
- watchlist:manage

### Reviewer Role
- grants:read
- grants:report
- milestones:read
- milestones:approve
- milestones:reject
- milestones:comment
- profiles:read
- profiles:update_own
- analytics:read

### Admin Role
- admin:all (includes all permissions)

## RBAC Middleware

Located in `src/middlewares/rbac-middleware.ts`

### Usage

```typescript
import { createRbacMiddleware } from "../middlewares/rbac-middleware";
import { RbacService } from "../services/rbac-service";

const rbacService = new RbacService(userRepo, roleRepo, userRoleRepo);
const { requirePermission, requireAnyPermission, requireAdmin, requireReviewerOrAdmin, requireGranteeOrAdmin } = createRbacMiddleware(rbacService);

// Require a specific permission
router.post("/grants", requirePermission("grants:create" as Permission), handler);

// Require any of multiple permissions
router.patch("/milestones/:id/approve", requireAnyPermission(["milestones:approve", "admin:all"] as Permission[]), handler);

// Convenience methods
router.post("/admin/sync", requireAdmin(), handler);
router.patch("/milestones/:id/approve", requireReviewerOrAdmin(), handler);
router.post("/grants", requireGranteeOrAdmin(), handler);
```

## RBAC Service

Located in `src/services/rbac-service.ts`

### Key Methods

- `getUserRoles(userId)`: Get all roles for a user
- `getUserRolesByAddress(stellarAddress)`: Get roles by stellar address
- `getUserPermissions(userId)`: Get all permissions for a user
- `getUserPermissionsByAddress(stellarAddress)`: Get permissions by address
- `userHasPermission(userId, permission)`: Check if user has permission
- `userHasPermissionByAddress(stellarAddress, permission)`: Check by address
- `assignRole(userId, roleName, assignedBy)`: Assign role to user
- `assignRoleByAddress(stellarAddress, roleName, assignedBy)`: Assign by address
- `revokeRole(userId, roleName)`: Revoke role from user
- `revokeRoleByAddress(stellarAddress, roleName)`: Revoke by address
- `initializeDefaultRoles()`: Initialize default roles in database

## Role Management API

Located in `src/routes/roles.ts`

### Endpoints

All endpoints are protected by admin middleware.

- `GET /roles` - Get all available roles
- `GET /roles/permissions` - Get all available permissions
- `GET /roles/user/:address` - Get roles for a specific user
- `GET /roles/user/:address/permissions` - Get permissions for a specific user
- `POST /roles/user/:address/assign` - Assign a role to a user
  - Body: `{ roleName: "user" | "grantee" | "reviewer" | "admin" }`
- `DELETE /roles/user/:address/revoke` - Revoke a role from a user
  - Body: `{ roleName: "user" | "grantee" | "reviewer" | "admin" }`
- `POST /roles/initialize` - Initialize default roles in database

## Migration Guide

### Old Pattern (Ad-hoc checks)

```typescript
const isPlatformAdmin = (address?: string) => !!address && env.adminAddresses.includes(address);

router.post("/admin/sync", async (req, res, next) => {
  const adminAddress = req.header("x-admin-address");
  if (!isPlatformAdmin(adminAddress)) {
    res.status(403).json({ error: "Admin privileges required" });
    return;
  }
  // ... handler logic
});
```

### New Pattern (RBAC middleware)

```typescript
const { requireAdmin } = createRbacMiddleware(rbacService);

router.post("/admin/sync", requireAdmin(), async (req, res, next) => {
  // ... handler logic - admin check is done by middleware
});
```

## Database Migration

To add the RBAC tables to your database, run:

```bash
# TypeORM will automatically create the tables on startup if sync is enabled
# Or run migrations if using migration mode
npm run migration:run
```

## Initialization

Default roles are automatically initialized on application startup via the `initializeDefaultRoles()` method called in `app.ts`.

## Testing

Unit tests for the RBAC middleware are located in `tests/unit/rbac-middleware.test.ts`.

Run tests with:
```bash
npm test
```

## Security Notes

1. The RBAC middleware checks the `x-user-address` header or `req.user.stellarAddress` for authentication
2. Admin endpoints are still protected by the existing admin middleware for signature verification
3. Role assignments are tracked with `assignedBy` field for audit purposes
4. Admin role has `admin:all` permission which grants access to all permissions
