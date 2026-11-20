/**
 * Funding percentage shared by FundingProgress and GrantStats so both
 * display the same value for the same grant.
 *
 * Computed in BigInt with two decimals of precision (round half up),
 * clamped to [0, 100].
 */
export function fundingPercent(funded: bigint, target: bigint): number {
  if (target <= 0n || funded <= 0n) return 0;
  const hundredths = (funded * 10000n + target / 2n) / target;
  return Math.min(100, Number(hundredths) / 100);
}
