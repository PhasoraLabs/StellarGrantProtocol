import { Router } from "express";
import { DataSource, Repository } from "typeorm";
import { Dispute } from "../entities/Dispute";
import { Grant } from "../entities/Grant";
import { Milestone } from "../entities/Milestone";
import { Activity } from "../entities/Activity";
import { RbacService } from "../services/rbac-service";
import { SignatureService } from "../services/signature-service";
import { WebhookDispatcher } from "../services/webhook-dispatcher";
import { walletLimiters } from "../middlewares/rate-limiter";
import { buildAuthMiddleware } from "../middlewares/auth-middleware";
import { NonceService } from "../services/nonce-service";
import type { AuthenticatedRequest } from "../types/auth";

const MAX_SKEW_MS = 5 * 60 * 1000;

const isUniqueViolation = (err: unknown): boolean => {
  const e = err as { code?: string; message?: string } | undefined;
  return (
    e?.code === "23505" ||
    e?.code === "SQLITE_CONSTRAINT" ||
    /unique constraint|UNIQUE constraint/i.test(e?.message ?? "")
  );
};

interface ResolveBody {
  approve_payout: boolean;
  tx_hash?: string;
  resolver_address: string;
  nonce: string;
  timestamp: number;
  signature: string;
}

interface ArgumentBody {
  role: "contributor" | "funder";
  argument: string;
  address?: string;
}

export const buildDisputesRouter = (
  dataSource: DataSource,
  rbacService: RbacService,
  webhookDispatcher: WebhookDispatcher,
  signatureService: SignatureService,
  nonceService: NonceService,
) => {
  const router = Router();
  const authMiddleware = buildAuthMiddleware(signatureService, nonceService);
  const disputeRepo: Repository<Dispute> = dataSource.getRepository(Dispute);
  const grantRepo: Repository<Grant> = dataSource.getRepository(Grant);
  const milestoneRepo: Repository<Milestone> =
    dataSource.getRepository(Milestone);
  const activityRepo: Repository<Activity> = dataSource.getRepository(Activity);

  // GET /disputes — list open disputes (?status=all for all)
  router.get("/", async (req, res, next) => {
    try {
      const statusFilter = req.query.status === "all" ? undefined : "open";

      const qb = disputeRepo
        .createQueryBuilder("d")
        .leftJoin(Grant, "g", "g.id = d.grantId")
        .leftJoin(
          Milestone,
          "m",
          "m.grantId = d.grantId AND m.idx = d.milestoneIdx",
        )
        .select([
          "d.id AS id",
          "d.grantId AS grantId",
          "d.milestoneIdx AS milestoneIdx",
          "d.status AS status",
          "d.openedAt AS openedAt",
          "d.resolvedAt AS resolvedAt",
          "g.title AS grantTitle",
          "m.title AS milestoneTitle",
        ]);

      if (statusFilter) {
        qb.where("d.status = :status", { status: statusFilter });
      }

      const disputes = await qb.getRawMany();
      res.json({ data: disputes });
    } catch (err) {
      next(err);
    }
  });

  // GET /disputes/:id — single dispute with full grant + milestone data
  router.get("/:id", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) {
        res.status(400).json({ error: "Invalid dispute id" });
        return;
      }

      const dispute = await disputeRepo.findOne({ where: { id } });
      if (!dispute) {
        res.status(404).json({ error: "Dispute not found" });
        return;
      }

      const [grant, milestone] = await Promise.all([
        grantRepo.findOne({ where: { id: dispute.grantId } }),
        milestoneRepo.findOne({
          where: { grantId: dispute.grantId, idx: dispute.milestoneIdx },
        }),
      ]);

      res.json({ data: { ...dispute, grant, milestone } });
    } catch (err) {
      next(err);
    }
  });

  // POST /disputes/:id/argument — submit contributor or funder argument
  router.post(
    "/:id/argument",
    authMiddleware,
    walletLimiters.disputeArgument,
    async (req: AuthenticatedRequest, res, next) => {
      try {
        const id = Number(req.params.id);
        if (Number.isNaN(id)) {
          res.status(400).json({ error: "Invalid dispute id" });
          return;
        }

        const address = req.user?.stellarAddress;
        if (!address) {
          res.status(401).json({ error: "Authentication required" });
          return;
        }

        const body = req.body as ArgumentBody;
        if (!body.role || !body.argument) {
          res.status(400).json({ error: "role and argument are required" });
          return;
        }
        // Reject attempts to act on behalf of another wallet.
        if (body.address && body.address !== address) {
          res
            .status(403)
            .json({ error: "Address does not match authenticated wallet" });
          return;
        }

        const dispute = await disputeRepo.findOne({ where: { id } });
        if (!dispute) {
          res.status(404).json({ error: "Dispute not found" });
          return;
        }

        if (dispute.status !== "open") {
          res
            .status(400)
            .json({ error: "Cannot add argument to a resolved dispute" });
          return;
        }

        const grant = await grantRepo.findOne({
          where: { id: dispute.grantId },
        });
        if (!grant) {
          res.status(404).json({ error: "Grant not found" });
          return;
        }

        // The authenticated wallet must actually hold the role it claims.
        if (body.role === "contributor") {
          if (address !== grant.recipient) {
            res
              .status(403)
              .json({ error: "Wallet is not the grant recipient" });
            return;
          }
          if (dispute.contributorArgument) {
            res
              .status(409)
              .json({ error: "Contributor argument already submitted" });
            return;
          }
          dispute.contributorArgument = body.argument;
        } else if (body.role === "funder") {
          const funded = await activityRepo.findOne({
            where: {
              type: "grant_funded",
              entityType: "grant",
              entityId: grant.id,
              actorAddress: address,
            },
          });
          if (address !== grant.owner && !funded) {
            res
              .status(403)
              .json({ error: "Wallet is not a funder of this grant" });
            return;
          }
          if (dispute.funderArgument) {
            res
              .status(409)
              .json({ error: "Funder argument already submitted" });
            return;
          }
          dispute.funderArgument = body.argument;
        } else {
          res
            .status(400)
            .json({ error: "role must be 'contributor' or 'funder'" });
          return;
        }

        const saved = await disputeRepo.save(dispute);
        res.json({ data: saved });
      } catch (err) {
        next(err);
      }
    },
  );

  // POST /disputes/:id/resolve — record resolution (council-only via RBAC + signed intent)
  router.post("/:id/resolve", async (req, res, next) => {
    try {
      const body = req.body as ResolveBody;
      const { resolver_address, nonce, timestamp, signature } = body;

      // All signature fields are required — no trust-on-header shortcut
      if (!resolver_address || !nonce || !timestamp || !signature) {
        res.status(401).json({
          error: "Authentication required",
          message:
            "resolver_address, nonce, timestamp, and signature are required",
        });
        return;
      }

      // Timestamp skew check (±5 minutes)
      if (Math.abs(Date.now() - timestamp) > MAX_SKEW_MS) {
        res.status(401).json({ error: "Expired intent timestamp" });
        return;
      }

      const id = Number(req.params.id);
      if (Number.isNaN(id)) {
        res.status(400).json({ error: "Invalid dispute id" });
        return;
      }

      if (body.approve_payout === undefined) {
        res.status(400).json({ error: "approve_payout is required" });
        return;
      }

      // Verify Ed25519 signature over the dispute resolve intent before any RBAC check
      const validSig = signatureService.verifyDisputeResolve({
        disputeId: id,
        approvePayout: body.approve_payout,
        resolverAddress: resolver_address,
        nonce,
        timestamp,
        signature,
      });

      if (!validSig) {
        res.status(401).json({ error: "Invalid signature" });
        return;
      }

      // RBAC check: only council members (admin:all) may resolve disputes
      const isCouncil = await rbacService.userHasPermissionByAddress(
        resolver_address,
        "admin:all",
      );

      if (!isCouncil) {
        res.status(403).json({
          error: "Forbidden",
          message: "Council member access required",
        });
        return;
      }

      const dispute = await disputeRepo.findOne({ where: { id } });
      if (!dispute) {
        res.status(404).json({ error: "Dispute not found" });
        return;
      }

      if (dispute.status !== "open") {
        res.status(400).json({ error: "Dispute is already resolved" });
        return;
      }

      dispute.status = body.approve_payout
        ? "resolved_payout"
        : "resolved_refund";
      dispute.resolvedBy = resolver_address;
      dispute.resolutionTxHash = body.tx_hash ?? null;
      dispute.resolvedAt = new Date();

      const saved = await disputeRepo.save(dispute);

      const [grant, milestone] = await Promise.all([
        grantRepo.findOne({ where: { id: dispute.grantId } }),
        milestoneRepo.findOne({
          where: { grantId: dispute.grantId, idx: dispute.milestoneIdx },
        }),
      ]);

      await webhookDispatcher
        .dispatch("dispute_resolved" as never, {
          disputeId: saved.id,
          grantId: saved.grantId,
          milestoneIdx: saved.milestoneIdx,
          status: saved.status,
          resolvedBy: saved.resolvedBy,
          txHash: saved.resolutionTxHash,
          grant: grant ? { id: grant.id, title: grant.title } : null,
          milestone: milestone
            ? { idx: milestone.idx, title: milestone.title }
            : null,
        })
        .catch(() => {
          // webhook failures must never break the response
        });

      res.json({ data: saved });
    } catch (err) {
      next(err);
    }
  });

  return router;
};

// Registers grant-scoped dispute routes:
//   GET  /grants/:grantId/milestones/:idx/dispute
//   POST /grants/:grantId/milestones/:idx/dispute
export const buildGrantDisputesRouter = (dataSource: DataSource) => {
  const router = Router({ mergeParams: true });
  const disputeRepo: Repository<Dispute> = dataSource.getRepository(Dispute);
  const grantRepo: Repository<Grant> = dataSource.getRepository(Grant);

  router.get(
    "/grants/:grantId/milestones/:idx/dispute",
    async (req, res, next) => {
      try {
        const grantId = Number(req.params.grantId);
        const milestoneIdx = Number(req.params.idx);

        if (Number.isNaN(grantId) || Number.isNaN(milestoneIdx)) {
          res.status(400).json({ error: "Invalid parameters" });
          return;
        }

        const dispute = await disputeRepo.findOne({
          where: { grantId, milestoneIdx },
        });

        if (!dispute) {
          res
            .status(404)
            .json({ error: "Dispute not found for this milestone" });
          return;
        }

        res.json({ data: dispute });
      } catch (err) {
        next(err);
      }
    },
  );

  router.post(
    "/grants/:grantId/milestones/:idx/dispute",
    async (req, res, next) => {
      try {
        const grantId = Number(req.params.grantId);
        const milestoneIdx = Number(req.params.idx);

        if (Number.isNaN(grantId) || Number.isNaN(milestoneIdx)) {
          res.status(400).json({ error: "Invalid parameters" });
          return;
        }

        const grant = await grantRepo.findOne({ where: { id: grantId } });
        if (!grant) {
          res.status(404).json({ error: "Grant not found" });
          return;
        }

        const existing = await disputeRepo.findOne({
          where: { grantId, milestoneIdx },
        });
        if (existing) {
          res
            .status(409)
            .json({ error: "Dispute already exists for this milestone" });
          return;
        }

        const dispute = disputeRepo.create({
          grantId,
          milestoneIdx,
          status: "open",
        });

        const saved = await disputeRepo.save(dispute);
        res.status(201).json({ data: saved });
      } catch (err) {
        // A concurrent request won the race; the unique constraint rejected ours.
        if (isUniqueViolation(err)) {
          res
            .status(409)
            .json({ error: "Dispute already exists for this milestone" });
          return;
        }
        next(err);
      }
    },
  );

  return router;
};
