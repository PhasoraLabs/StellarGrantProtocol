/**
 * Fresh grant funding snapshot used to re-validate contributions before submit.
 */

export interface GrantFundingState {
  budget: bigint;
  funded: bigint;
}

/**
 * Re-fetch the grant's budget/funded amounts through the Next.js API proxy.
 * Returns null when the grant cannot be loaded.
 */
export async function fetchGrantFundingState(
  grantId: string,
): Promise<GrantFundingState | null> {
  const res = await fetch(`/api/grants/${encodeURIComponent(grantId)}`, {
    cache: "no-store",
  });
  if (!res.ok) return null;

  const json = (await res.json()) as {
    grant?: { budget?: string | number | bigint; funded?: string | number | bigint };
  };
  if (!json.grant) return null;

  try {
    return {
      budget: BigInt(json.grant.budget ?? 0),
      funded: BigInt(json.grant.funded ?? 0),
    };
  } catch {
    return null;
  }
}

/** Remaining unfunded capacity; clamped at 0 when already over-subscribed. */
export function remainingCapacity(state: GrantFundingState): bigint {
  const left = state.budget - state.funded;
  return left > 0n ? left : 0n;
}
