import { Router } from "express";
import { Repository } from "typeorm";
import { z } from "zod";
import { User } from "../entities/User";
import { Role, RoleName } from "../entities/Role";
import { UserRole } from "../entities/UserRole";
import { RbacService } from "../services/rbac-service";
import { validateBody, validateParams } from "../middlewares/validation-middleware";
import { createRbacMiddleware, AuthenticatedRequest } from "../middlewares/rbac-middleware";
import { addressParamSchema } from "../schemas";

const assignRoleSchema = z.object({
  roleName: z.enum(["user", "grantee", "reviewer", "admin"]),
});

export const buildRolesRouter = (
  userRepo: Repository<User>,
  roleRepo: Repository<Role>,
  userRoleRepo: Repository<UserRole>,
  rbacService: RbacService,
) => {
  const router = Router();
  const { requirePermission } = createRbacMiddleware(rbacService);

  /**
   * GET /roles
   * Get all available roles
   */
  router.get("/", async (_req, res, next) => {
    try {
      const roles = await roleRepo.find({ order: { name: "ASC" } });
      res.json({ data: roles });
    } catch (error) {
      next(error);
    }
  });

  /**
   * GET /roles/permissions
   * Get all available permissions
   */
  router.get("/permissions", async (_req, res, next) => {
    try {
      const { PERMISSIONS } = await import("../config/rbac");
      res.json({ data: PERMISSIONS });
    } catch (error) {
      next(error);
    }
  });

  /**
   * GET /roles/user/:address
   * Get roles for a specific user
   */
  router.get("/user/:address", validateParams(addressParamSchema), async (req, res, next) => {
    try {
      const { address } = (req as any).validatedParams;
      const roles = await rbacService.getUserRolesByAddress(address);
      res.json({ data: roles });
    } catch (error) {
      next(error);
    }
  });

  /**
   * GET /roles/user/:address/permissions
   * Get permissions for a specific user
   */
  router.get("/user/:address/permissions", validateParams(addressParamSchema), async (req, res, next) => {
    try {
      const { address } = (req as any).validatedParams;
      const permissions = await rbacService.getUserPermissionsByAddress(address);
      res.json({ data: permissions });
    } catch (error) {
      next(error);
    }
  });

  /**
   * POST /roles/user/:address/assign
   * Assign a role to a user (admin only)
   */
  router.post(
    "/user/:address/assign",
    requirePermission("users:assign_roles" as any),
    validateParams(addressParamSchema),
    validateBody(assignRoleSchema),
    async (req: AuthenticatedRequest, res, next) => {
      try {
        const { address } = (req as any).validatedParams;
        const { roleName } = (req as any).validatedBody;
        const assignedBy = req.user?.stellarAddress || (req as any).adminAddress;

        const userRole = await rbacService.assignRoleByAddress(
          address,
          roleName as RoleName,
          assignedBy,
        );

        res.status(201).json({ data: userRole });
      } catch (error: any) {
        if (error.message === "User not found") {
          res.status(404).json({ error: "User not found" });
          return;
        }
        if (error.message === "Role not found") {
          res.status(404).json({ error: "Role not found" });
          return;
        }
        if (error.message === "User already has this role") {
          res.status(409).json({ error: "User already has this role" });
          return;
        }
        next(error);
      }
    }
  );

  /**
   * DELETE /roles/user/:address/revoke
   * Revoke a role from a user (admin only)
   */
  router.delete(
    "/user/:address/revoke",
    requirePermission("users:assign_roles" as any),
    validateParams(addressParamSchema),
    validateBody(assignRoleSchema),
    async (req: AuthenticatedRequest, res, next) => {
      try {
        const { address } = (req as any).validatedParams;
        const { roleName } = (req as any).validatedBody;

        await rbacService.revokeRoleByAddress(address, roleName as RoleName);

        res.json({ success: true });
      } catch (error: any) {
        if (error.message === "User not found") {
          res.status(404).json({ error: "User not found" });
          return;
        }
        if (error.message === "Role not found") {
          res.status(404).json({ error: "Role not found" });
          return;
        }
        next(error);
      }
    }
  );

  /**
   * POST /roles/initialize
   * Initialize default roles in the database (admin only)
   */
  router.post(
    "/initialize",
    requirePermission("admin:all" as any),
    async (_req, res, next) => {
      try {
        await rbacService.initializeDefaultRoles();
        res.json({ success: true, message: "Default roles initialized" });
      } catch (error) {
        next(error);
      }
    }
  );

  return router;
};
