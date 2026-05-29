import type { Grant } from "@/types";
import { mapStatusToNumber } from "@/lib/grants/api";

export type ApiGrantListItem = {
  id: number;
  title: string;
  description?: string | null;
  status: string | number;
  recipient?: string;
  totalAmount?: string;
  funded?: string | number;
  budget?: string | number;
  deadline?: string | number | bigint;
  updatedAt?: string;
  token?: string;
};

export function mapListItemToGrant(raw: ApiGrantListItem, funded = 0n): Grant {
  const deadline =
    raw.deadline !== undefined && raw.deadline !== ""
      ? typeof raw.deadline === "string"
        ? BigInt(Math.floor(new Date(raw.deadline).getTime() / 1000))
        : BigInt(raw.deadline)
      : 0n;

  return {
    id: String(raw.id),
    owner: raw.recipient ?? "",
    recipient: raw.recipient ?? "",
    title: raw.title,
    description: raw.description ?? "",
    budget: BigInt(raw.totalAmount ?? raw.budget ?? "0"),
    funded: typeof raw.funded === "bigint" ? raw.funded : BigInt(raw.funded ?? funded),
    deadline,
    status: mapStatusToNumber(raw.status),
    milestones: 0,
    reviewers: [],
    created_at: raw.updatedAt
      ? BigInt(Math.floor(new Date(raw.updatedAt).getTime() / 1000))
      : 0n,
    token: raw.token ?? "native",
  };
}

export function addressToColor(addr: string): string {
  let hash = 0;
  for (const char of addr) hash = char.charCodeAt(0) + ((hash << 5) - hash);
  return `hsl(${Math.abs(hash) % 360}, 70%, 50%)`;
}

export function matchesQuery(text: string, query: string): boolean {
  const words = query
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);
  if (words.length === 0) return true;
  const haystack = text.toLowerCase();
  return words.every((word) => haystack.includes(word));
}
