import { DataSource, Repository } from "typeorm";
import { Grant } from "../entities/Grant";
import { Contributor } from "../entities/Contributor";
import { ReputationLog } from "../entities/ReputationLog";

/** A leaderboard row. */
export interface LeaderboardEntry {
  address: string;
  reputation: number;
  totalGrantsCompleted: number;
}

export class LeaderboardService {
  private readonly contributorRepo: Repository<Contributor>;
  private readonly reputationLogRepo: Repository<ReputationLog>;

  constructor(private readonly dataSource: DataSource) {
    this.contributorRepo = this.dataSource.getRepository(Contributor);
    this.reputationLogRepo = this.dataSource.getRepository(ReputationLog);
  }

  // ── Offset-based (backwards-compatible) ─────────────────────────────────────

  async getLeaderboard(
    period: "all-time" | "monthly",
    page: number = 1,
    limit: number = 20,
  ): Promise<[LeaderboardEntry[], number]> {
    if (period === "all-time") {
      return this._allTimeLeaderboard(page, limit);
    }
    return this._monthlyLeaderboard(page, limit);
  }

  // ── Cursor-based ─────────────────────────────────────────────────────────────

  /**
   * Returns leaderboard entries after the given cursor address,
   * ordered by reputation DESC then address ASC for stability.
   * Fetches `limit + 1` rows so the caller can detect `hasMore`.
   *
   * The cursor encodes the last-seen address (Contributor's PK).
   */
  async getLeaderboardAfterCursor(
    period: "all-time" | "monthly",
    afterAddress: string,
    limit: number = 20,
  ): Promise<[LeaderboardEntry[], number]> {
    if (period === "all-time") {
      return this._allTimeLeaderboardCursor(afterAddress, limit);
    }
    return this._monthlyLeaderboardCursor(afterAddress, limit);
  }

  // ── Private: all-time ────────────────────────────────────────────────────────

  private async _allTimeLeaderboard(
    page: number,
    limit: number,
  ): Promise<[LeaderboardEntry[], number]> {
    const [rows, total] = await this.contributorRepo.findAndCount({
      where: { isBlacklisted: false },
      order: { reputation: "DESC", address: "ASC" },
      skip: (page - 1) * limit,
      take: limit,
    });

    const entries: LeaderboardEntry[] = rows.map((c) => ({
      address: c.address,
      reputation: c.reputation,
      totalGrantsCompleted: c.totalGrantsCompleted,
    }));
    return [entries, total];
  }

  private async _allTimeLeaderboardCursor(
    afterAddress: string,
    limit: number,
  ): Promise<[LeaderboardEntry[], number]> {
    // Find the reputation of the cursor address so we can do a proper keyset
    const cursorRow = await this.contributorRepo.findOne({
      where: { address: afterAddress },
    });

    const total = await this.contributorRepo.count({ where: { isBlacklisted: false } });

    const qb = this.contributorRepo
      .createQueryBuilder("c")
      .where("c.isBlacklisted = false")
      .orderBy("c.reputation", "DESC")
      .addOrderBy("c.address", "ASC")
      .take(limit + 1);

    if (cursorRow) {
      // Keyset: rows with lower reputation, or same reputation but address > cursor
      qb.andWhere(
        "(c.reputation < :rep OR (c.reputation = :rep AND c.address > :addr))",
        { rep: cursorRow.reputation, addr: afterAddress },
      );
    }

    const rows = await qb.getMany();
    const entries: LeaderboardEntry[] = rows.map((c) => ({
      address: c.address,
      reputation: c.reputation,
      totalGrantsCompleted: c.totalGrantsCompleted,
    }));
    return [entries, total];
  }

  // ── Private: monthly ─────────────────────────────────────────────────────────

  private async _monthlyLeaderboard(
    page: number,
    limit: number,
  ): Promise<[LeaderboardEntry[], number]> {
    const thirtyDaysAgo = new Date();
    thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);

    const countResult = await this.reputationLogRepo
      .createQueryBuilder("log")
      .select("COUNT(DISTINCT log.address)", "count")
      .where("log.timestamp >= :thirtyDaysAgo", { thirtyDaysAgo })
      .getRawOne<{ count: string }>();

    const totalCount = parseInt(countResult?.count ?? "0", 10);
    if (totalCount === 0) {
      return this._allTimeLeaderboard(page, limit);
    }

    const rawResults = await this.reputationLogRepo
      .createQueryBuilder("log")
      .select("log.address", "address")
      .addSelect("SUM(log.gain)", "monthlyReputation")
      .where("log.timestamp >= :thirtyDaysAgo", { thirtyDaysAgo })
      .groupBy("log.address")
      .orderBy("SUM(log.gain)", "DESC")
      .addOrderBy("log.address", "ASC")
      .offset((page - 1) * limit)
      .limit(limit)
      .getRawMany<{ address: string; monthlyReputation: string }>();

    const entries = await Promise.all(
      rawResults.map(async (row) => {
        const contributor = await this.contributorRepo.findOne({
          where: { address: row.address },
        });
        return {
          address: row.address,
          reputation: parseInt(row.monthlyReputation, 10),
          totalGrantsCompleted: contributor?.totalGrantsCompleted ?? 0,
        };
      }),
    );

    return [entries, totalCount];
  }

  private async _monthlyLeaderboardCursor(
    afterAddress: string,
    limit: number,
  ): Promise<[LeaderboardEntry[], number]> {
    const thirtyDaysAgo = new Date();
    thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);

    const countResult = await this.reputationLogRepo
      .createQueryBuilder("log")
      .select("COUNT(DISTINCT log.address)", "count")
      .where("log.timestamp >= :thirtyDaysAgo", { thirtyDaysAgo })
      .getRawOne<{ count: string }>();

    const totalCount = parseInt(countResult?.count ?? "0", 10);
    if (totalCount === 0) {
      return this._allTimeLeaderboardCursor(afterAddress, limit);
    }

    // Find the monthly reputation of the cursor address for keyset
    const cursorResult = await this.reputationLogRepo
      .createQueryBuilder("log")
      .select("SUM(log.gain)", "monthlyReputation")
      .where("log.address = :addr AND log.timestamp >= :thirtyDaysAgo", {
        addr: afterAddress,
        thirtyDaysAgo,
      })
      .getRawOne<{ monthlyReputation: string }>();

    const cursorRep = parseInt(cursorResult?.monthlyReputation ?? "0", 10);

    const rawResults = await this.reputationLogRepo
      .createQueryBuilder("log")
      .select("log.address", "address")
      .addSelect("SUM(log.gain)", "monthlyReputation")
      .where("log.timestamp >= :thirtyDaysAgo", { thirtyDaysAgo })
      .groupBy("log.address")
      .having(
        "(SUM(log.gain) < :rep OR (SUM(log.gain) = :rep AND log.address > :addr))",
        { rep: cursorRep, addr: afterAddress },
      )
      .orderBy("SUM(log.gain)", "DESC")
      .addOrderBy("log.address", "ASC")
      .limit(limit + 1)
      .getRawMany<{ address: string; monthlyReputation: string }>();

    const entries = await Promise.all(
      rawResults.map(async (row) => {
        const contributor = await this.contributorRepo.findOne({
          where: { address: row.address },
        });
        return {
          address: row.address,
          reputation: parseInt(row.monthlyReputation, 10),
          totalGrantsCompleted: contributor?.totalGrantsCompleted ?? 0,
        };
      }),
    );

    return [entries, totalCount];
  }
}
