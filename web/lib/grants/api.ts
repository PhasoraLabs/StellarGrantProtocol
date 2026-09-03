/**
 * Grant API helpers — map indexing API responses to frontend Grant types.
 */

import { API_URL } from "@/lib/constants";
import type { Grant, Milestone } from "@/types";

export interface ApiEnrichedMilestone {
  idx: number;
  title: string;
  description?: string | null;
  deadline: string;
  submitted?: boolean;
  approved?: boolean;
  paid?: boolean;
  proof_hash?: string | null;
  submitted_at?: string | null;
  approved_at?: string | null;
  paid_at?: string | null;
  overdue?: boolean;
  daysUntilDeadline?: number;
  token?: string;
  amount?: string | bigint;
}

export interface ApiGrantDetail {
  id: number;
  title: string;
  description?: string | null;
  status: string;
  recipient: string;
  totalAmount: string;
  updatedAt?: string;
  isWatched?: boolean;
  milestones?: ApiEnrichedMilestone[];
  milestoneSummary?: {
    total: number;
    submitted: number;
    overdue: number;
  };
}

export interface GrantDetailPayload {
  grant: Grant;
  milestones: Milestone[];
  completedMilestones: number;
  isWatched: boolean;
}

export interface FunderRow {
  address: string;
  amount: bigint;
}

const STATUS_MAP: Record<string, number> = {
  pending: 0,
  active: 1,
  review: 2,
  in_progress: 2,
  "in progress": 2,
  completed: 3,
  complete: 3,
  cancelled: 4,
  canceled: 4,
};

export function mapStatusToNumber(status: string | number): number {
  if (typeof status === "number") return status;
  return STATUS_MAP[status.toLowerCase()] ?? 0;
}

function parseDeadlineIso(iso: string): bigint {
  return BigInt(Math.floor(new Date(iso).getTime() / 1000));
}

export function mapApiMilestone(raw: ApiEnrichedMilestone): Milestone {
  return {
    idx: raw.idx,
    title: raw.title,
    description: raw.description ?? "",
    proof_hash: raw.proof_hash ?? null,
    submitted: raw.submitted ?? false,
    approved: raw.approved ?? false,
    paid: raw.paid ?? false,
    submitted_at: raw.submitted_at ? parseDeadlineIso(raw.submitted_at) : null,
    approved_at: raw.approved_at ? parseDeadlineIso(raw.approved_at) : null,
    paid_at: raw.paid_at ? parseDeadlineIso(raw.paid_at) : null,
    overdue: raw.overdue,
    daysUntilDeadline: raw.daysUntilDeadline,
    token: raw.token,
    amount: raw.amount !== undefined ? BigInt(raw.amount) : undefined,
  };
}

export function mapApiGrantDetail(data: ApiGrantDetail, funded = 0n): GrantDetailPayload {
  const milestoneList = (data.milestones ?? []).map(mapApiMilestone);

  const sortedDeadlines = (data.milestones ?? [])
    .map((m) => m.deadline)
    .filter(Boolean)
    .sort((a, b) => new Date(a).getTime() - new Date(b).getTime());
  const grantDeadline =
    sortedDeadlines.length > 0 ? parseDeadlineIso(sortedDeadlines[sortedDeadlines.length - 1]!) : 0n;

  const grant: Grant = {
    id: String(data.id),
    owner: data.recipient,
    recipient: data.recipient,
    title: data.title,
    description: data.description ?? "",
    budget: BigInt(data.totalAmount || "0"),
    funded,
    deadline: grantDeadline,
    status: mapStatusToNumber(data.status),
    milestones: milestoneList.length,
    reviewers: [],
    created_at: data.updatedAt
      ? BigInt(Math.floor(new Date(data.updatedAt).getTime() / 1000))
      : 0n,
    token: "native",
  };

  const completedMilestones =
    data.milestoneSummary?.submitted ??
    milestoneList.filter((m) => m.approved || m.paid).length;

  return {
    grant,
    milestones: milestoneList,
    completedMilestones,
    isWatched: data.isWatched ?? false,
  };
}

export async function fetchGrantById(id: string): Promise<GrantDetailPayload | null> {
  const res = await fetch(`${API_URL}/grants/${id}`, {
    next: { revalidate: 60 },
  });
  if (!res.ok) return null;
  const json = (await res.json()) as { data?: ApiGrantDetail };
  if (!json.data) return null;

  let funded = 0n;
  try {
    const funders = await fetchGrantFunders(id);
    funded = funders.reduce((sum, f) => sum + f.amount, 0n);
  } catch {
    funded = 0n;
  }

  const payload = mapApiGrantDetail(json.data, funded);
  const reviewers = await fetchGrantReviewers(id);
  payload.grant.reviewers = reviewers;
  return payload;
}

export async function fetchGrantReviewers(grantId: string): Promise<string[]> {
  const res = await fetch(`${API_URL}/grant_reviewers/grant/${grantId}`, {
    next: { revalidate: 60 },
  });
  if (!res.ok) return [];
  const json = (await res.json()) as {
    data?: Array<{ reviewerStellarAddress: string }>;
  };
  return (json.data ?? []).map((r) => r.reviewerStellarAddress);
}

export async function fetchGrantFunders(grantId: string): Promise<FunderRow[]> {
  const res = await fetch(`${API_URL}/grants/${grantId}/funders`, {
    next: { revalidate: 30 },
  });
  if (!res.ok) return [];
  const json = (await res.json()) as {
    data?: Array<{ address: string; amount: string }>;
  };
  return (json.data ?? []).map((f) => ({
    address: f.address,
    amount: BigInt(f.amount),
  }));
}
