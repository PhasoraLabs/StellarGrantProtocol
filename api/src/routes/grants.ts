import { Router } from "express";
import { In, Repository } from "typeorm";
import { Grant } from "../entities/Grant";
import { Milestone } from "../entities/Milestone";
import { MilestoneProof } from "../entities/MilestoneProof";
import { UserWatchlist } from "../entities/UserWatchlist";
import { Activity } from "../entities/Activity";
import { PlatformConfig } from "../entities/PlatformConfig";
import { FeeCollection } from "../entities/FeeCollection";
import { GrantSyncService } from "../services/grant-sync-service";
import { SignatureService } from "../services/signature-service";
import { ResponseCacheService, responseCacheKeys } from "../services/response-cache";
import { Contributor } from "../entities/Contributor";
import { z } from "zod";
import { createProofLookup, enrichMilestone, summarizeMilestones } from "../utils/milestones";

// ---------------------------------------------------------------------------
// Query-param validation helpers
// ---------------------------------------------------------------------------

const VALID_SORT_FIELDS = ["updatedAt", "totalAmount", "id"] as const;
type SortField = (typeof VALID_SORT_FIELDS)[number];

const VALID_SORT_ORDERS = ["ASC", "DESC"] as const;
type SortOrder = (typeof VALID_SORT_ORDERS)[number];

function parsePagination(pageStr: unknown, limitStr: unknown) {
  const page = parseInt(String(pageStr ?? "1"), 10);
  const limit = parseInt(String(limitStr ?? "20"), 10);

  if (!Number.isFinite(page) || page < 1)
    return { error: "page must be a positive integer" };
  if (!Number.isFinite(limit) || limit < 1 || limit > 100)
    return { error: "limit must be between 1 and 100" };

  return { page, limit };
}

function parseSortField(raw: unknown): SortField {
  const candidate = String(raw ?? "").trim();
  return VALID_SORT_FIELDS.includes(candidate as SortField)
    ? (candidate as SortField)
    : "id";
}

function parseSortOrder(raw: unknown): SortOrder {
  const candidate = String(raw ?? "").toUpperCase().trim();
  return VALID_SORT_ORDERS.includes(candidate as SortOrder)
    ? (candidate as SortOrder)
    : "ASC";
}

/**
 * tags: supports comma-separated or repeated query params
 */
function parseTags(raw: unknown): string[] {
  if (!raw) return [];

  const values = Array.isArray(raw) ? raw : [raw];

  return [...new Set(
    values
      .flatMap((v) => String(v).split(","))
      .map((t) => t.trim().toLowerCase())
      .filter(Boolean)
  )];
}

function getPreferredLanguage(header: string | undefined): string {
  if (!header) return "en";
  const langs = header.split(",").map(l => l.split(";")[0].trim().toLowerCase());
  return langs[0] || "en";
}

function localizeGrant(grant: Grant, lang: string) {
  const metadata = grant.localizedMetadata || {};
  const translation = metadata[lang] || metadata["en"] || {};
  
  return {
    ...grant,
    title: translation.title || grant.title,
    description: translation.description || null,
  };
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

const watchSchema = z.object({
  address: z.string().min(10).max(120),
  signature: z.string().min(32),
  nonce: z.string().min(8).max(80),
  timestamp: z.number().int().positive(),
});

export const buildGrantRouter = (
  grantRepo: Repository<Grant>,
  milestoneRepo: Repository<Milestone>,
  proofRepo: Repository<MilestoneProof>,
  syncService: GrantSyncService,
  signatureService: SignatureService,
  responseCache: ResponseCacheService,
) => {
  const router = Router();
  const watchlistRepo = grantRepo.manager.getRepository(UserWatchlist);
  const activityRepo = grantRepo.manager.getRepository(Activity);
  const configRepo = grantRepo.manager.getRepository(PlatformConfig);
  const feeRepo = grantRepo.manager.getRepository(FeeCollection);
  const contributorRepo = grantRepo.manager.getRepository(Contributor);

  const contributorProfilesByAddress = async (addresses: string[]) => {
    const unique = [...new Set(addresses.map((a) => a.trim()).filter(Boolean))];
    if (unique.length === 0) return new Map<string, Contributor>();
    const rows = await contributorRepo.findBy(unique.map((address) => ({ address })));
    return new Map(rows.map((c) => [c.address, c]));
  };

  const toProfile = (c: Contributor | undefined) => c ? ({
    address: c.address,
    bio: c.bio ?? null,
    profilePictureUrl: c.profilePictureUrl ?? null,
    githubUrl: c.githubUrl ?? null,
    twitterUrl: c.twitterUrl ?? null,
    linkedinUrl: c.linkedinUrl ?? null,
    updatedAt: c.updatedAt,
  }) : null;

  router.get("/", async (req, res, next) => {
    try {
      const lang = getPreferredLanguage(req.header("accept-language"));
      const userAddress = req.header("x-user-address");

      // ---------------- Pagination ----------------
      const pagination = parsePagination(req.query.page, req.query.limit);
      if ("error" in pagination) {
        res.status(400).json({ error: pagination.error });
        return;
      }

      const { page, limit } = pagination;

      // ---------------- Sorting ----------------
      const sortBy = parseSortField(req.query.sortBy);
      const order = parseSortOrder(req.query.order);

      // ---------------- Filters ----------------
      const statusFilter = req.query.status
        ? String(req.query.status).trim().toLowerCase()
        : null;

      const funderFilter = req.query.funder
        ? String(req.query.funder).trim()
        : null;

      const tagsFilter = parseTags(req.query.tags);
      const communityId = req.query.communityId ? Number(req.query.communityId) : null;
      if (communityId !== null && Number.isNaN(communityId)) {
        res.status(400).json({ error: "communityId must be a valid number" });
        return;
      }

      const canUseListCache =
        responseCache.isEnabled() &&
        !userAddress &&
        page === 1 &&
        limit === 20 &&
        !statusFilter &&
        !funderFilter &&
        tagsFilter.length === 0 &&
        !communityId &&
        sortBy === "id" &&
        order === "ASC";

      if (canUseListCache) {
        const cached = await responseCache.get(responseCacheKeys.grantsFirstPage(lang));
        if (cached) {
          res.type("application/json").send(cached);
          return;
        }
      }

      await syncService.syncAllGrants();

      // ---------------- Query Builder ----------------
      const qb = grantRepo.createQueryBuilder("grant");
      
      qb.where("grant.isFlagged = false");

      if (statusFilter) {
        qb.andWhere("LOWER(grant.status) = :status", { status: statusFilter });
      }

      if (funderFilter) {
        qb.andWhere("LOWER(grant.recipient) LIKE :funder", {
          funder: `%${funderFilter.toLowerCase()}%`,
        });
      }

      if (communityId !== null) {
        qb.andWhere("grant.communityId = :communityId", { communityId });
      }

      /**
       * AND tag logic: every requested tag must appear in the grant's tags column.
       * One andWhere per tag so all conditions must be satisfied simultaneously.
       */
      if (tagsFilter.length > 0) {
        tagsFilter.forEach((tag, idx) => {
          qb.andWhere(
            "LOWER(COALESCE(grant.tags, '')) LIKE :tag" + idx,
            { ["tag" + idx]: `%${tag}%` },
          );
        });
      }

      // ---------------- Sorting ----------------
      // totalAmount is stored as varchar so we must cast to a number before
      // sorting, otherwise "9000" sorts after "10000" lexicographically.
      if (sortBy === "totalAmount") {
        qb.orderBy("CAST(grant.totalAmount AS REAL)", order);
      } else {
        qb.orderBy(`grant.${sortBy}`, order);
      }

      qb.skip((page - 1) * limit).take(limit);

      // ---------------- Execute ----------------
      const [data, total] = await qb.getManyAndCount();
      const profiles = await contributorProfilesByAddress(data.map((g) => g.recipient));

      // Add isWatched flag if user address is provided
      let watchedGrantIds: Set<number> = new Set();
      if (userAddress) {
        const watchlistEntries = await watchlistRepo.find({
          where: { address: userAddress },
          select: ["grantId"],
        });
        watchedGrantIds = new Set(watchlistEntries.map(e => e.grantId));
      }

      const grantIds = data.map((grant) => grant.id);
      const milestones = grantIds.length > 0
        ? await milestoneRepo.find({
            where: { grantId: In(grantIds) },
            order: { deadline: "ASC", idx: "ASC" },
          })
        : [];
      const proofs = grantIds.length > 0
        ? await proofRepo.find({
            where: { grantId: In(grantIds) },
            select: {
              grantId: true,
              milestoneIdx: true,
              createdAt: true,
            },
          })
        : [];
      const proofLookup = createProofLookup(proofs);
      const milestonesByGrantId = new Map<number, ReturnType<typeof enrichMilestone>[]>();

      for (const milestone of milestones) {
        const enriched = enrichMilestone(milestone, proofLookup.get(`${milestone.grantId}:${milestone.idx}`));
        const current = milestonesByGrantId.get(milestone.grantId) ?? [];
        current.push(enriched);
        milestonesByGrantId.set(milestone.grantId, current);
      }

      const responseData = data.map(g => {
        const summary = summarizeMilestones(milestonesByGrantId.get(g.id) || []);
        return {
          ...localizeGrant(g, lang),
          isWatched: watchedGrantIds.has(g.id),
          recipientProfile: toProfile(profiles.get(g.recipient)),
          milestoneSummary: summary,
          hasOverdueMilestones: summary.overdue > 0,
        };
      });

      const payload = {
        data: responseData,
        meta: {
          total,
          page,
          limit,
          totalPages: Math.ceil(total / limit),
        },
      };
      const body = JSON.stringify(payload);
      if (canUseListCache) {
        await responseCache.set(responseCacheKeys.grantsFirstPage(lang), body);
      }
      res.type("application/json").send(body);
    } catch (error) {
      next(error);
    }
  });

  // ---------------- Top funders for a grant ----------------
  router.get("/:id/funders", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) {
        res.status(400).json({ error: "Invalid grant id" });
        return;
      }

      const collections = await feeRepo.find({
        where: { grantId: id },
        order: { createdAt: "DESC" },
      });

      const byAddress = new Map<string, bigint>();
      for (const row of collections) {
        const prev = byAddress.get(row.funderAddress) ?? 0n;
        byAddress.set(row.funderAddress, prev + BigInt(row.totalContribution));
      }

      const funders = [...byAddress.entries()]
        .map(([address, amount]) => ({ address, amount: amount.toString() }))
        .sort((a, b) => (BigInt(b.amount) > BigInt(a.amount) ? 1 : BigInt(b.amount) < BigInt(a.amount) ? -1 : 0))
        .slice(0, 5);

      res.json({ data: funders });
    } catch (error) {
      next(error);
    }
  });

  // ---------------- Single Grant ----------------
  router.get("/:id", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) {
        res.status(400).json({ error: "Invalid grant id" });
        return;
      }

      await syncService.syncGrant(id);
      const lang = getPreferredLanguage(req.header("accept-language"));

      const grant = await grantRepo.findOne({ where: { id }, relations: { community: true } });

      if (!grant) {
        res.status(404).json({ error: "Grant not found" });
        return;
      }
      const recipientProfile = await contributorRepo.findOne({ where: { address: grant.recipient } });

      const userAddress = req.header("x-user-address");
      let isWatched = false;
      if (userAddress) {
        const watchlistEntry = await watchlistRepo.findOne({
          where: { address: userAddress, grantId: id },
        });
        isWatched = !!watchlistEntry;
      }

      const milestones = await milestoneRepo.find({
        where: { grantId: id },
        order: { deadline: "ASC", idx: "ASC" },
      });
      const proofs = await proofRepo.find({
        where: { grantId: id },
        select: { grantId: true, milestoneIdx: true, createdAt: true },
      });
      const proofLookup = createProofLookup(proofs);
      const enrichedMilestones = milestones.map(m => enrichMilestone(m, proofLookup.get(`${m.grantId}:${m.idx}`)));

      const summary = summarizeMilestones(enrichedMilestones);

      res.json({
        data: {
          ...localizeGrant(grant, lang),
          isWatched,
          recipientProfile: toProfile(recipientProfile ?? undefined),
          milestones: enrichedMilestones,
          milestoneSummary: summary,
          hasOverdueMilestones: summary.overdue > 0,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  // ---------------- Grant History ----------------
  router.get("/:id/history", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) {
        res.status(400).json({ error: "Invalid grant id" });
        return;
      }

      const { GrantHistory } = await import("../entities/GrantHistory");
      const historyRepo = grantRepo.manager.getRepository(GrantHistory);
      const history = await historyRepo.find({
        where: { grantId: id },
        order: { createdAt: "DESC" },
      });

      res.json({ data: history });
    } catch (error) {
      next(error);
    }
  });

  // ---------------- Export CSV / PDF ----------------
  router.get("/:id/export/csv", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) return res.status(400).json({ error: "Invalid grant id" });

      const grant = await grantRepo.findOne({ where: { id } });
      if (!grant) return res.status(404).json({ error: "Grant not found" });

      const milestones = await milestoneRepo.find({ where: { grantId: id } });
      const { exportService } = await import("../services/export-service");
      const csvData = await exportService.exportCsv(grant, milestones, []);

      res.header("Content-Type", "text/csv");
      res.attachment(`grant_${id}_export.csv`);
      res.send(csvData);
    } catch (error) {
      next(error);
    }
  });

  router.get("/:id/export/pdf", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) return res.status(400).json({ error: "Invalid grant id" });

      const grant = await grantRepo.findOne({ where: { id } });
      if (!grant) return res.status(404).json({ error: "Grant not found" });

      const milestones = await milestoneRepo.find({ where: { grantId: id } });
      const { exportService } = await import("../services/export-service");
      const pdfBuffer = await exportService.exportPdf(grant, milestones, []);

      res.header("Content-Type", "application/pdf");
      res.attachment(`grant_${id}_export.pdf`);
      res.send(pdfBuffer);
    } catch (error) {
      next(error);
    }
  });

  // ---------------- Report Grant ----------------
  router.post("/:id/report", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) return res.status(400).json({ error: "Invalid grant id" });

      const { reporterAddress, reason } = req.body;
      if (!reporterAddress || !reason) {
        return res.status(400).json({ error: "Missing reporterAddress or reason" });
      }

      const grant = await grantRepo.findOne({ where: { id } });
      if (!grant) return res.status(404).json({ error: "Grant not found" });

      const { Report } = await import("../entities/Report");
      const reportRepo = grantRepo.manager.getRepository(Report);

      await reportRepo.save({
        grantId: id,
        reporterAddress,
        reason,
        status: "pending"
      });

      // Check if threshold reached
      const reportCount = await reportRepo.count({ where: { grantId: id, status: "pending" } });
      if (reportCount >= 5 && !grant.isFlagged) {
        grant.isFlagged = true;
        await grantRepo.save(grant);
      }

      res.status(201).json({ ok: true });
    } catch (error) {
      next(error);
    }
  });

  // ---------------- Watch Grant ----------------
  router.post("/:id/watch", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) {
        res.status(400).json({ error: "Invalid grant id" });
        return;
      }

      const parsed = watchSchema.safeParse(req.body);
      if (!parsed.success) {
        res.status(400).json({ error: "Invalid payload", details: parsed.error.issues });
        return;
      }

      const { address, signature, nonce, timestamp } = parsed.data;
      const maxSkewMs = 5 * 60 * 1000;
      if (Math.abs(Date.now() - timestamp) > maxSkewMs) {
        res.status(400).json({ error: "Expired intent timestamp" });
        return;
      }

      // Verify signature
      const action = `POST:/grants/${id}/watch`;
      const message = [
        "stellargrant:user_action:v1",
        address,
        nonce,
        timestamp,
        action,
      ].join("|");

      const { StrKey, Keypair } = await import("@stellar/stellar-sdk");
      if (!StrKey.isValidEd25519PublicKey(address)) {
        res.status(400).json({ error: "Invalid Stellar address" });
        return;
      }

      const keypair = Keypair.fromPublicKey(address);
      const isValid = keypair.verify(
        Buffer.from(message, "utf8"),
        Buffer.from(signature, "base64"),
      );

      if (!isValid) {
        res.status(401).json({ error: "Invalid signature" });
        return;
      }

      // Check if grant exists
      const grant = await grantRepo.findOne({ where: { id } });
      if (!grant) {
        res.status(404).json({ error: "Grant not found" });
        return;
      }

      // Add to watchlist
      await watchlistRepo.save({
        address,
        grantId: id,
      });

      // Log activity for watchlist addition
      await grantRepo.manager.getRepository(UserWatchlist).manager.getRepository(Activity).save({
        type: "watchlist_added",
        entityType: "grant",
        entityId: id,
        actorAddress: address,
        data: null,
      });

      res.status(201).json({ data: { watched: true } });
    } catch (error: any) {
      if (error?.code === "23505" || error?.code === "SQLITE_CONSTRAINT") {
        res.status(409).json({ error: "Grant already watched" });
        return;
      }
      next(error);
    }
  });

  // ---------------- Unwatch Grant ----------------
  router.delete("/:id/watch", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) {
        res.status(400).json({ error: "Invalid grant id" });
        return;
      }

      const parsed = watchSchema.safeParse(req.body);
      if (!parsed.success) {
        res.status(400).json({ error: "Invalid payload", details: parsed.error.issues });
        return;
      }

      const { address, signature, nonce, timestamp } = parsed.data;
      const maxSkewMs = 5 * 60 * 1000;
      if (Math.abs(Date.now() - timestamp) > maxSkewMs) {
        res.status(400).json({ error: "Expired intent timestamp" });
        return;
      }

      // Verify signature
      const action = `DELETE:/grants/${id}/watch`;
      const message = [
        "stellargrant:user_action:v1",
        address,
        nonce,
        timestamp,
        action,
      ].join("|");

      const { StrKey, Keypair } = await import("@stellar/stellar-sdk");
      if (!StrKey.isValidEd25519PublicKey(address)) {
        res.status(400).json({ error: "Invalid Stellar address" });
        return;
      }

      const keypair = Keypair.fromPublicKey(address);
      const isValid = keypair.verify(
        Buffer.from(message, "utf8"),
        Buffer.from(signature, "base64"),
      );

      if (!isValid) {
        res.status(401).json({ error: "Invalid signature" });
        return;
      }

      // Remove from watchlist
      const result = await watchlistRepo.delete({
        address,
        grantId: id,
      });

      if (result.affected === 0) {
        res.status(404).json({ error: "Watchlist entry not found" });
        return;
      }

      // Log activity for watchlist removal
      await activityRepo.save({
        type: "watchlist_removed",
        entityType: "grant",
        entityId: id,
        actorAddress: address,
        data: null,
      });

      res.json({ data: { watched: false } });
    } catch (error) {
      next(error);
    }
  });

  // ---------------- Fund Grant (Sync/Record) ----------------
  router.post("/:id/fund", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) {
        res.status(400).json({ error: "Invalid grant id" });
        return;
      }

      const { funderAddress, amount } = req.body;
      if (!funderAddress || !amount) {
        res.status(400).json({ error: "Missing funderAddress or amount" });
        return;
      }

      const config = await configRepo.findOne({ where: { key: "platform_fee_percentage" } });
      const percentage = config ? parseFloat(config.value) : 0;

      // Calculate fee
      const feeAmount = (BigInt(amount) * BigInt(Math.floor(percentage * 100))) / BigInt(10000);

      await feeRepo.save({
        grantId: id,
        funderAddress,
        totalContribution: amount,
        feeAmount: feeAmount.toString(),
        feePercentage: percentage.toString(),
      });

      // Log activity
      await activityRepo.save({
        type: "grant_funded" as any,
        entityType: "grant",
        entityId: id,
        actorAddress: funderAddress,
        data: { amount, feeAmount: feeAmount.toString(), feePercentage: percentage },
      });

      res.json({ ok: true, feeAmount: feeAmount.toString(), feePercentage: percentage });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
