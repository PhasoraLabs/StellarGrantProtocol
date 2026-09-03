import { Milestone } from "../entities/Milestone";
import { MilestoneProof } from "../entities/MilestoneProof";

const DAY_IN_MS = 24 * 60 * 60 * 1000;
const UPCOMING_REMINDER_WINDOWS = new Set([7, 3, 1]);

export type EnrichedMilestone = {
  id: number;
  grantId: number;
  idx: number;
  title: string;
  description: string | null;
  deadline: string;
  submitted: boolean;
  submittedAt: string | null;
  overdue: boolean;
  daysUntilDeadline: number;
};

export type MilestoneSummary = {
  total: number;
  submitted: number;
  overdue: number;
  upcoming: number;
  nextDeadline: string | null;
};

export const createProofLookup = (proofs: Pick<MilestoneProof, "grantId" | "milestoneIdx" | "createdAt">[]) =>
  new Map(
    proofs.map((proof) => [
      `${proof.grantId}:${proof.milestoneIdx}`,
      proof,
    ]),
  );

const startOfUtcDay = (value: Date) =>
  Date.UTC(value.getUTCFullYear(), value.getUTCMonth(), value.getUTCDate());

export const getDaysUntilDeadline = (deadlineIso: string, now = new Date()) =>
  Math.round((startOfUtcDay(new Date(deadlineIso)) - startOfUtcDay(now)) / DAY_IN_MS);

export const isMilestoneOverdue = (deadlineIso: string, submitted: boolean, now = new Date()) =>
  !submitted && new Date(deadlineIso).getTime() < now.getTime();

export const getReminderWindow = (deadlineIso: string, submitted: boolean, now = new Date()) => {
  if (submitted) return null;
  const daysUntilDeadline = getDaysUntilDeadline(deadlineIso, now);
  return UPCOMING_REMINDER_WINDOWS.has(daysUntilDeadline) ? daysUntilDeadline : null;
};

export const enrichMilestone = (
  milestone: Milestone,
  proof?: Pick<MilestoneProof, "createdAt">,
  now = new Date(),
): EnrichedMilestone => {
  const submitted = !!proof;
  return {
    id: milestone.id,
    grantId: milestone.grantId,
    idx: milestone.idx,
    title: milestone.title,
    description: milestone.description,
    deadline: milestone.deadline,
    submitted,
    submittedAt: proof?.createdAt?.toISOString?.() ?? null,
    overdue: isMilestoneOverdue(milestone.deadline, submitted, now),
    daysUntilDeadline: getDaysUntilDeadline(milestone.deadline, now),
  };
};

export const summarizeMilestones = (milestones: EnrichedMilestone[]): MilestoneSummary => {
  const pendingMilestones = milestones.filter((milestone) => !milestone.submitted);
  const nextDeadline = pendingMilestones
    .slice()
    .sort((a, b) => new Date(a.deadline).getTime() - new Date(b.deadline).getTime())[0]?.deadline ?? null;

  return {
    total: milestones.length,
    submitted: milestones.filter((milestone) => milestone.submitted).length,
    overdue: milestones.filter((milestone) => milestone.overdue).length,
    upcoming: milestones.filter(
      (milestone) => !milestone.submitted && !milestone.overdue && milestone.daysUntilDeadline <= 7,
    ).length,
    nextDeadline,
  };
};
