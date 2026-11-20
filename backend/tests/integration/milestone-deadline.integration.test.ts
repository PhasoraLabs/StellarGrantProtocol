import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { DataSource } from "typeorm";
import { buildDataSource } from "../../src/db/data-source";
import { Contributor } from "../../src/entities/Contributor";
import { Milestone } from "../../src/entities/Milestone";
import { GrantSyncService } from "../../src/services/grant-sync-service";
import { MilestoneDeadlineService } from "../../src/services/milestone-deadline-service";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";

const DAY_IN_MS = 24 * 60 * 60 * 1000;

describe("Milestone Deadline Integration", () => {
  let dataSource: DataSource;
  const sorobanClient = new MockSorobanContractClient();

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  it("sends upcoming and overdue milestone alerts exactly once per milestone state", async () => {
    const syncService = new GrantSyncService(dataSource, sorobanClient);
    await syncService.syncAllGrants();

    const contributorRepo = dataSource.getRepository(Contributor);
    await contributorRepo.save({
      address: "GBRPYHIL2C2WBO36G6UIGR2PA4M3TQ7VOY3RTMAL4LRRA67ZOHQ65SZD",
      reputation: 0,
      totalGrantsCompleted: 0,
      isBlacklisted: false,
      email: "creator@example.com",
      emailNotifications: true,
    });

    const milestoneRepo = dataSource.getRepository(Milestone);
    const milestones = await milestoneRepo.find({
      where: { grantId: 1 },
      order: { idx: "ASC" },
    });
    const firstDeadline = new Date(milestones[0].deadline);
    const fixedNow = new Date(firstDeadline.getTime() - 7 * DAY_IN_MS + 60 * 60 * 1000);

    const send = vi.fn(async () => undefined);
    const notifyUser = vi.fn();
    const service = new MilestoneDeadlineService(dataSource, {
      emailSender: { send },
      notifyUser,
      now: () => fixedNow,
    });

    const firstRun = await service.runCheck();
    expect(firstRun).toEqual({ remindersSent: 3, overdueAlertsSent: 1 });
    expect(send).toHaveBeenCalledTimes(4);
    expect(notifyUser).toHaveBeenCalledTimes(4);

    const milestoneZero = await milestoneRepo.findOneByOrFail({ grantId: 1, idx: 0 });
    const milestoneThree = await milestoneRepo.findOneByOrFail({ grantId: 1, idx: 3 });
    expect(milestoneZero.lastDeadlineReminderDaysBefore).toBe(7);
    expect(milestoneThree.overdueNotifiedAt).not.toBeNull();

    const secondRun = await service.runCheck();
    expect(secondRun).toEqual({ remindersSent: 0, overdueAlertsSent: 0 });
    expect(send).toHaveBeenCalledTimes(4);
    expect(notifyUser).toHaveBeenCalledTimes(4);
  });
});
