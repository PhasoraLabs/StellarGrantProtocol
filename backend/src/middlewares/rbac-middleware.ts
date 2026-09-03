import { Request, Response, NextFunction } from "express";
import { Permission } from "../config/rbac";
import { RbacService } from "../services/rbac-service";

/**
 * Extended Request type with user information 
 */
export interface AuthenticatedRequest extends Request {
  user?: {
    id?: number;
    stellarAddress?: string;
  };
}

/**
 * Create RBAC middleware factory
 */
export const createRbacMiddleware = (rbacService: RbacService) => {
  /**
   * Middleware to check if user has a specific permission
   */
  const requirePermission = (permission: Permission) => {
    return async (req: AuthenticatedRequest, res: Response, next: NextFunction) => {
      try {
        const stellarAddress = req.user?.stellarAddress || req.header("x-user-address");
        
        if (!stellarAddress) {
          res.status(401).json({ error: "Authentication required" });
          return;
        }

        const hasPermission = await rbacService.userHasPermissionByAddress(stellarAddress, permission);
        
        if (!hasPermission) {
          res.status(403).json({ 
            error: "Forbidden", 
            message: `Permission '${permission}' required` 
          });
          return;
        }

        next();
      } catch (error) {
        console.error("RBAC middleware error:", error);
        res.status(500).json({ error: "Internal server error" });
      }
    };
  };

  /**
   * Middleware to check if user has any of the specified permissions
   */
  const requireAnyPermission = (permissions: Permission[]) => {
    return async (req: AuthenticatedRequest, res: Response, next: NextFunction) => {
      try {
        const stellarAddress = req.user?.stellarAddress || req.header("x-user-address");
        
        if (!stellarAddress) {
          res.status(401).json({ error: "Authentication required" });
          return;
        }

        const hasPermission = await rbacService.userHasAnyPermissionByAddress(stellarAddress, permissions);
        
        if (!hasPermission) {
          res.status(403).json({ 
            error: "Forbidden", 
            message: `One of the following permissions required: ${permissions.join(", ")}` 
          });
          return;
        }

        next();
      } catch (error) {
        console.error("RBAC middleware error:", error);
        res.status(500).json({ error: "Internal server error" });
      }
    };
  };

  /**
   * Middleware to check if user is admin (convenience method)
   */
  const requireAdmin = () => {
    return requirePermission("admin:all" as Permission);
  };

  /**
   * Middleware to check if user is a reviewer or admin
   */
  const requireReviewerOrAdmin = () => {
    return requireAnyPermission(["milestones:approve", "admin:all"] as Permission[]);
  };

  /**
   * Middleware to check if user is a grantee or admin
   */
  const requireGranteeOrAdmin = () => {
    return requireAnyPermission(["grants:create", "admin:all"] as Permission[]);
  };

  /**
   * Middleware to check resource ownership
   * This is a helper that checks if the user owns the resource they're trying to access
   */
  const requireOwnership = (getResourceOwnerId: (req: AuthenticatedRequest) => string | number | Promise<string | number>) => {
    return async (req: AuthenticatedRequest, res: Response, next: NextFunction) => {
      try {
        const stellarAddress = req.user?.stellarAddress || req.header("x-user-address");
        
        if (!stellarAddress) {
          res.status(401).json({ error: "Authentication required" });
          return;
        }

        const resourceOwnerId = await getResourceOwnerId(req);
        
        // If user is admin, allow access
        const isAdmin = await rbacService.userHasPermissionByAddress(stellarAddress, "admin:all" as Permission);
        if (isAdmin) {
          return next();
        }

        // Check if user owns the resource
        if (resourceOwnerId !== stellarAddress) {
          res.status(403).json({ 
            error: "Forbidden", 
            message: "You do not own this resource" 
          });
          return;
        }

        next();
      } catch (error) {
        console.error("Ownership check error:", error);
        res.status(500).json({ error: "Internal server error" });
      }
    };
  };

  return {
    requirePermission,
    requireAnyPermission,
    requireAdmin,
    requireReviewerOrAdmin,
    requireGranteeOrAdmin,
    requireOwnership,
  };
};
