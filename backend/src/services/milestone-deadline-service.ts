import { DataSource, In } from "typeorm";
import { Contributor } from "../entities/Contributor";
import { Grant } from "../entities/Grant";
import { Milestone } from "../entities/Milestone";
import { MilestoneProof } from "../entities/MilestoneProof";
import { EmailPayload, emailService } from "./email-service";
import { logger } from "../config/logger";
import { notificationService } from "./notification-service";
import { createProofLookup, getReminderWindow, isMilestoneOverdue } from "../utils/milestones";

type EmailSender = {
  send(payload: EmailPayload): Promise<void>;
};

type DeadlineServiceDeps = {
  emailSender?: EmailSender;
  now?: () => Date;
  notifyUser?: (address: string, type: string, payload: Record<string, unknown>) => void;
};

export class MilestoneDeadlineService {
  private timer: ReturnType<typeof setInterval> | null = null;
  private readonly emailSender: EmailSender;
  private readonly now: () => Date;
  private readonly notifyUser: (address: string, type: string, payload: Record<string, unknown>) => void;

  constructor(
    private readonly dataSource: DataSource,
    deps: DeadlineServiceDeps = {},
  ) {
    this.emailSender = deps.emailSender ?? emailService;
    this.now = deps.now ?? (() => new Date());
    this.notifyUser = deps.notifyUser ?? ((address, type, payload) => notificationService.notifyUser(address, type, payload));
  }

  start(intervalMs = 24 * 60 * 60 * 1000): void {
    if (this.timer) return;

    void this.runCheck();
    this.timer = setInterval(() => {
      void this.runCheck();
    }, intervalMs);
  }

  stop(): void {
    if (!this.timer) return;
    clearInterval(this.timer);
    this.timer = null;
  }

  async runCheck(): Promise<{ remindersSent: number; overdueAlertsSent: number }> {
    const now = this.now();
    const milestoneRepo = this.dataSource.getRepository(Milestone);
    const proofRepo = this.dataSource.getRepository(MilestoneProof);
    const contributorRepo = this.dataSource.getRepository(Contributor);
    const grantRepo = this.dataSource.getRepository(Grant);

    const milestones = await milestoneRepo.find();
    if (milestones.length === 0) {
      return { remindersSent: 0, overdueAlertsSent: 0 };
    }

    const grantIds = [...new Set(milestones.map((milestone) => milestone.grantId))];
    const grants = grantIds.length > 0
      ? await grantRepo.findBy({ id: In(grantIds) })
      : [];
    const proofs = await proofRepo.find({
      select: {
        grantId: true,
        milestoneIdx: true,
        createdAt: true,
      },
    });

    const grantMap = new Map(grants.map((grant) => [grant.id, grant]));
    const proofLookup = createProofLookup(proofs);
    const recipientAddresses = [...new Set(grants.map((grant) => grant.recipient))];
    const contributors = recipientAddresses.length > 0
      ? await contributorRepo.findBy({ address: In(recipientAddresses) })
      : [];
    const contributorMap = new Map(contributors.map((contributor) => [contributor.address, contributor]));

    let remindersSent = 0;
    let overdueAlertsSent = 0;

    for (const milestone of milestones) {
      const grant = grantMap.get(milestone.grantId);
      if (!grant) continue;

      const proof = proofLookup.get(`${milestone.grantId}:${milestone.idx}`);
      const submitted = !!proof;
      if (submitted) {
        continue;
      }

      const reminderWindow = getReminderWindow(milestone.deadline, submitted, now);
      if (reminderWindow !== null) {
        const alreadySentForWindow =
          milestone.lastDeadlineReminderDaysBefore === reminderWindow &&
          milestone.lastDeadlineReminderAt !== null;

        if (!alreadySentForWindow) {
          const payload = {
            grantId: grant.id,
            grantTitle: grant.title,
            milestoneIdx: milestone.idx,
            milestoneTitle: milestone.title,
            daysRemaining: reminderWindow,
            deadline: milestone.deadline,
          };
          await this.dispatchDeadlineAlert(grant.recipient, payload, "milestone_deadline_upcoming", contributorMap);
          milestone.lastDeadlineReminderAt = now.toISOString();
          milestone.lastDeadlineReminderDaysBefore = reminderWindow;
          await milestoneRepo.save(milestone);
          remindersSent += 1;
        }
        continue;
      }

      if (isMilestoneOverdue(milestone.deadline, submitted, now) && !milestone.overdueNotifiedAt) {
        const payload = {
          grantId: grant.id,
          grantTitle: grant.title,
          milestoneIdx: milestone.idx,
          milestoneTitle: milestone.title,
          deadline: milestone.deadline,
          daysOverdue: Math.max(1, Math.abs(Math.round((new Date(milestone.deadline).getTime() - now.getTime()) / (24 * 60 * 60 * 1000)))),
        };
        await this.dispatchDeadlineAlert(grant.recipient, payload, "milestone_deadline_overdue", contributorMap);
        milestone.overdueNotifiedAt = now.toISOString();
        await milestoneRepo.save(milestone);
        overdueAlertsSent += 1;
      }
    }

    logger.info("Milestone deadline sweep completed", { remindersSent, overdueAlertsSent });
    return { remindersSent, overdueAlertsSent };
  }

  private async dispatchDeadlineAlert(
    recipient: string,
    payload: Record<string, unknown>,
    event: "milestone_deadline_upcoming" | "milestone_deadline_overdue",
    contributorMap: Map<string, Contributor>,
  ) {
    this.notifyUser(recipient, event, payload);

    const contributor = contributorMap.get(recipient);
    if (!contributor?.email || contributor.emailNotifications === false) {
      return;
    }

    try {
      await this.emailSender.send({
        to: contributor.email,
        event,
        data: payload as Record<string, string | number>,
      });
    } catch (error) {
      logger.error("Failed to send milestone deadline email", {
        recipient,
        event,
        error,
      });
    }
  }
}
