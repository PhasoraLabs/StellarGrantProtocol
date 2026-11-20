import { describe, it, expect, beforeEach, vi } from "vitest";
import { RbacService } from "../../src/services/rbac-service";
import { createRbacMiddleware, AuthenticatedRequest } from "../../src/middlewares/rbac-middleware";
import { Permission, ROLE_PERMISSIONS } from "../../src/config/rbac";
import { Repository } from "typeorm";
import { User } from "../../src/entities/User";
import { Role, RoleName } from "../../src/entities/Role";
import { UserRole } from "../../src/entities/UserRole";

describe("RBAC Middleware", () => {
  let rbacService: RbacService;
  let userRepo: Repository<User>;
  let roleRepo: Repository<Role>;
  let userRoleRepo: Repository<UserRole>;
  let mockRequest: Partial<AuthenticatedRequest>;
  let mockResponse: any;
  let mockNext: any;

  beforeEach(() => {
    // Mock repositories
    userRepo = {
      findOne: vi.fn(),
    } as any;

    roleRepo = {
      findOne: vi.fn(),
    } as any;

    userRoleRepo = {
      find: vi.fn(),
      delete: vi.fn(),
      create: vi.fn(),
      save: vi.fn(),
    } as any;

    rbacService = new RbacService(userRepo, roleRepo, userRoleRepo);

    // Mock request/response/next
    mockRequest = {
      header: vi.fn(),
      user: { stellarAddress: "test-address" },
    };

    mockResponse = {
      status: vi.fn().mockReturnThis(),
      json: vi.fn().mockReturnThis(),
    };

    mockNext = vi.fn();
  });

  describe("requirePermission", () => {
    it("should allow access when user has the required permission", async () => {
      const { requirePermission } = createRbacMiddleware(rbacService);

      // Mock user has permission
      vi.spyOn(rbacService, "userHasPermissionByAddress").mockResolvedValue(true);

      await requirePermission("grants:read" as Permission)(
        mockRequest as AuthenticatedRequest,
        mockResponse,
        mockNext
      );

      expect(mockNext).toHaveBeenCalled();
      expect(mockResponse.status).not.toHaveBeenCalled();
    });

    it("should deny access when user lacks the required permission", async () => {
      const { requirePermission } = createRbacMiddleware(rbacService);

      // Mock user lacks permission
      vi.spyOn(rbacService, "userHasPermissionByAddress").mockResolvedValue(false);

      await requirePermission("grants:create" as Permission)(
        mockRequest as AuthenticatedRequest,
        mockResponse,
        mockNext
      );

      expect(mockNext).not.toHaveBeenCalled();
      expect(mockResponse.status).toHaveBeenCalledWith(403);
      expect(mockResponse.json).toHaveBeenCalledWith({
        error: "Forbidden",
        message: "Permission 'grants:create' required",
      });
    });

    it("should return 401 when no stellar address is provided", async () => {
      const { requirePermission } = createRbacMiddleware(rbacService);

      mockRequest.header = vi.fn().mockReturnValue(undefined);
      mockRequest.user = undefined;

      await requirePermission("grants:read" as Permission)(
        mockRequest as AuthenticatedRequest,
        mockResponse,
        mockNext
      );

      expect(mockNext).not.toHaveBeenCalled();
      expect(mockResponse.status).toHaveBeenCalledWith(401);
      expect(mockResponse.json).toHaveBeenCalledWith({
        error: "Authentication required",
      });
    });
  });

  describe("requireAnyPermission", () => {
    it("should allow access when user has at least one of the required permissions", async () => {
      const { requireAnyPermission } = createRbacMiddleware(rbacService);

      vi.spyOn(rbacService, "userHasAnyPermissionByAddress").mockResolvedValue(true);

      await requireAnyPermission(["grants:read", "grants:create"] as Permission[])(
        mockRequest as AuthenticatedRequest,
        mockResponse,
        mockNext
      );

      expect(mockNext).toHaveBeenCalled();
    });

    it("should deny access when user has none of the required permissions", async () => {
      const { requireAnyPermission } = createRbacMiddleware(rbacService);

      vi.spyOn(rbacService, "userHasAnyPermissionByAddress").mockResolvedValue(false);

      await requireAnyPermission(["grants:create", "grants:delete"] as Permission[])(
        mockRequest as AuthenticatedRequest,
        mockResponse,
        mockNext
      );

      expect(mockNext).not.toHaveBeenCalled();
      expect(mockResponse.status).toHaveBeenCalledWith(403);
    });
  });

  describe("requireAdmin", () => {
    it("should allow access for admin users", async () => {
      const { requireAdmin } = createRbacMiddleware(rbacService);

      vi.spyOn(rbacService, "userHasPermissionByAddress").mockResolvedValue(true);

      await requireAdmin()(mockRequest as AuthenticatedRequest, mockResponse, mockNext);

      expect(mockNext).toHaveBeenCalled();
    });

    it("should deny access for non-admin users", async () => {
      const { requireAdmin } = createRbacMiddleware(rbacService);

      vi.spyOn(rbacService, "userHasPermissionByAddress").mockResolvedValue(false);

      await requireAdmin()(mockRequest as AuthenticatedRequest, mockResponse, mockNext);

      expect(mockNext).not.toHaveBeenCalled();
      expect(mockResponse.status).toHaveBeenCalledWith(403);
    });
  });

  describe("requireReviewerOrAdmin", () => {
    it("should allow access for reviewers", async () => {
      const { requireReviewerOrAdmin } = createRbacMiddleware(rbacService);

      vi.spyOn(rbacService, "userHasAnyPermissionByAddress").mockResolvedValue(true);

      await requireReviewerOrAdmin()(
        mockRequest as AuthenticatedRequest,
        mockResponse,
        mockNext
      );

      expect(mockNext).toHaveBeenCalled();
    });

    it("should allow access for admins", async () => {
      const { requireReviewerOrAdmin } = createRbacMiddleware(rbacService);

      vi.spyOn(rbacService, "userHasAnyPermissionByAddress").mockResolvedValue(true);

      await requireReviewerOrAdmin()(
        mockRequest as AuthenticatedRequest,
        mockResponse,
        mockNext
      );

      expect(mockNext).toHaveBeenCalled();
    });
  });

  describe("requireGranteeOrAdmin", () => {
    it("should allow access for grantees", async () => {
      const { requireGranteeOrAdmin } = createRbacMiddleware(rbacService);

      vi.spyOn(rbacService, "userHasAnyPermissionByAddress").mockResolvedValue(true);

      await requireGranteeOrAdmin()(
        mockRequest as AuthenticatedRequest,
        mockResponse,
        mockNext
      );

      expect(mockNext).toHaveBeenCalled();
    });
  });
});

describe("RBAC Configuration", () => {
  it("should have defined all required roles", () => {
    expect(ROLE_PERMISSIONS).toHaveProperty("user");
    expect(ROLE_PERMISSIONS).toHaveProperty("grantee");
    expect(ROLE_PERMISSIONS).toHaveProperty("reviewer");
    expect(ROLE_PERMISSIONS).toHaveProperty("admin");
  });

  it("should have admin role with all permissions", () => {
    const adminPerms = ROLE_PERMISSIONS[RoleName.ADMIN];
    expect(adminPerms).toContain("admin:all" as Permission);
  });

  it("should have user role with basic permissions", () => {
    const userPerms = ROLE_PERMISSIONS[RoleName.USER];
    expect(userPerms).toContain("grants:read" as Permission);
    expect(userPerms).toContain("profiles:update_own" as Permission);
    expect(userPerms).not.toContain("grants:create" as Permission);
  });

  it("should have grantee role with grant creation permissions", () => {
    const granteePerms = ROLE_PERMISSIONS[RoleName.GRANTEE];
    expect(granteePerms).toContain("grants:create" as Permission);
    expect(granteePerms).toContain("milestones:create" as Permission);
  });

  it("should have reviewer role with approval permissions", () => {
    const reviewerPerms = ROLE_PERMISSIONS[RoleName.REVIEWER];
    expect(reviewerPerms).toContain("milestones:approve" as Permission);
    expect(reviewerPerms).toContain("milestones:reject" as Permission);
  });
});
