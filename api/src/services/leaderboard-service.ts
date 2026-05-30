import { Grant } from "../entities/Grant";
import { Contributor } from "../entities/Contributor";
import { ReputationLog } from "../entities/ReputationLog";

export class LeaderboardService {
  private readonly contributorRepo: Repository<Contributor>;
  private readonly reputationLogRepo: Repository<ReputationLog>;

  constructor(private readonly dataSource: DataSource) {
    this.contributorRepo = this.dataSource.getRepository(Contributor);
    this.reputationLogRepo = this.dataSource.getRepository(ReputationLog);
  }

  async getLeaderboard(period: "all-time" | "monthly", page: number = 1, limit: number = 20) {
    if (period === "all-time") {
      // Compute leaderboard based on unique grant recipients
      const grants = await this.dataSource.getRepository(Grant).find();
      const recipientMap = new Map<string, number>();
      for (const grant of grants) {
        const count = recipientMap.get(grant.recipient) ?? 0;
        recipientMap.set(grant.recipient, count + 1);
      }
      const distinctRecipients = Array.from(recipientMap.entries()).map(([address, count]) => ({
        address,
        reputation: 100,
        totalGrantsCompleted: count,
      }));
      // Apply pagination
      const sorted = distinctRecipients.sort((a, b) => b.reputation - a.reputation);
      const paged = sorted.slice((page - 1) * limit, page * limit);
      return [paged, distinctRecipients.length];
    } else {

      // Monthly: Reputation gained in the last 30 days
      const thirtyDaysAgo = new Date();
      thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);

      const qb = this.reputationLogRepo.createQueryBuilder("log")
        .select("log.address", "address")
        .addSelect("SUM(log.gain)", "monthlyReputation")
        .where("log.timestamp >= :thirtyDaysAgo", { thirtyDaysAgo })
        .groupBy("log.address")
        .orderBy("SUM(log.gain)", "DESC")
        .offset((page - 1) * limit)
        .limit(limit);

      const counts = await this.reputationLogRepo.createQueryBuilder("log")
        .select("COUNT(DISTINCT log.address)", "count")
        .where("log.timestamp >= :thirtyDaysAgo", { thirtyDaysAgo })
        .getRawOne();

      const totalCount = parseInt(counts?.count ?? "0", 10);
      if (totalCount === 0) {
        return this.getLeaderboard("all-time", page, limit);
      }

      const rawResults = await qb.getRawMany();
      
      // Enrich with totalGrantsCompleted from Contributor entity
      const leaderboardData = await Promise.all(
        rawResults.map(async (res) => {
          const contributor = await this.contributorRepo.findOne({
            where: { address: res.address },
          });
          return {
            address: res.address,
            reputation: parseInt(res.monthlyReputation, 10),
            totalGrantsCompleted: contributor?.totalGrantsCompleted ?? 0,
          };
        })
      );

      return [leaderboardData, parseInt(counts.count, 10)];
    }
  }
}
