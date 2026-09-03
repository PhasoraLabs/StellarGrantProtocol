import type { Grant } from "@/types";

/** JSON-safe grant for API responses (bigint → string). */
export function serializeGrant(grant: Grant) {
  return {
    ...grant,
    budget: grant.budget.toString(),
    funded: grant.funded.toString(),
    deadline: grant.deadline.toString(),
    created_at: grant.created_at.toString(),
  };
}
