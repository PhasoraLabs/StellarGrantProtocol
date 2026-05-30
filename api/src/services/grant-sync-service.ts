import { DataSource, Repository } from "typeorm";
import { Grant } from "../entities/Grant";
import { Milestone } from "../entities/Milestone";
import { Contributor } from "../entities/Contributor";
import { ReputationLog } from "../entities/ReputationLog";
import { SorobanContractClient, SorobanGrant } from "../soroban/types";

export class GrantSyncService {
  private readonly grantRepo: Repository<Grant>;
  private readonly milestoneRepo: Repository<Milestone>;
  private readonly contributorRepo: Repository<Contributor>;
  private readonly reputationLogRepo: Repository<ReputationLog>;

  constructor(
    private readonly dataSource: DataSource,
    private readonly sorobanClient: SorobanContractClient,
  ) {
    this.grantRepo = this.dataSource.getRepository(Grant);
    this.milestoneRepo = this.dataSource.getRepository(Milestone);
    this.contributorRepo = this.dataSource.getRepository(Contributor);
    this.reputationLogRepo = this.dataSource.getRepository(ReputationLog);
  }

  async syncAllGrants(): Promise<void> {
    const grants = await this.sorobanClient.fetchGrants();
    for (const grant of grants) {
      const savedGrant = await this.syncGrantInternal(grant);
      await this.upsertMilestones(grant, savedGrant);
      await this.updateContributorReputation(savedGrant);
    }
  }

  async syncGrant(id: number): Promise<void> {
    const grant = await this.sorobanClient.fetchGrantById(id);
    if (!grant) return;
    const savedGrant = await this.syncGrantInternal(grant);
    await this.upsertMilestones(grant, savedGrant);
    await this.updateContributorReputation(savedGrant);
  }

  private async syncGrantInternal(grant: SorobanGrant): Promise<Grant> {
    return this.grantRepo.save(this.normalizeGrant(grant));
  }

  private async upsertMilestones(grant: SorobanGrant, savedGrant: Grant): Promise<void> {
    if (!grant.milestones?.length) {
      return;
    }

    await Promise.all(
      grant.milestones.map((milestone) =>
        this.milestoneRepo.upsert(
          {
            grantId: savedGrant.id,
            idx: milestone.idx,
            title: milestone.title,
            description: milestone.description ?? null,
            deadline: milestone.deadline,
          },
          ["grantId", "idx"],
        ),
      ),
    );
  }

  private async updateContributorReputation(grant: Grant): Promise<void> {
    const address = grant.recipient;
    let contributor = await this.contributorRepo.findOne({ where: { address } });

    if (!contributor) {
      contributor = this.contributorRepo.create({
        address,
        reputation: 0,
        totalGrantsCompleted: 0,
      });
    }

    // Check if we already logged reputation for this grant
    const existingLog = await this.reputationLogRepo.findOne({
      where: { address, gain: 100 },
      order: { timestamp: "DESC" },
    });

    // Only award reputation if it hasn't been logged yet
    if (!existingLog) {
      const reputationGain = 100;
      contributor.reputation = (contributor.reputation ?? 0) + reputationGain;
      await this.contributorRepo.save(contributor);

      // Log the reputation gain
      await this.reputationLogRepo.save({
        address,
        gain: reputationGain,
        timestamp: new Date(),
      });
    }
  }

  private normalizeGrant(grant: SorobanGrant): Partial<Grant> {
    const { milestones, ...grantData } = grant;
    return {
      ...grantData,
      owner: grant.owner ?? grant.recipient,
    };
  }
}
