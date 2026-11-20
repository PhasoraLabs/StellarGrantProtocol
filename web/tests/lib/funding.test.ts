import { describe, it, expect } from 'vitest';
import { fundingPercent } from '@/lib/utils/funding';

describe('fundingPercent', () => {
  it('keeps two decimals of precision', () => {
    expect(fundingPercent(2n, 3n)).toBe(66.67);
    expect(fundingPercent(1n, 3n)).toBe(33.33);
  });
  it('clamps to 100 and handles zero target', () => {
    expect(fundingPercent(20n, 10n)).toBe(100);
    expect(fundingPercent(5n, 0n)).toBe(0);
    expect(fundingPercent(0n, 10n)).toBe(0);
  });
});
