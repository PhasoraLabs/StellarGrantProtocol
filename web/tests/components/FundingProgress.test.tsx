import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FundingProgress } from '@/components/grants/FundingProgress';

vi.mock('@/lib/tokens', () => ({
  getTokenMetadata: vi.fn().mockResolvedValue({ symbol: 'XLM', decimals: 7 }),
  formatTokenAmount: vi.fn((amount: bigint) => String(Number(amount) / 1e7)),
}));

describe('FundingProgress', () => {
  it('shows 0% when current is 0', () => {
    render(<FundingProgress current={0n} target={10_000_000n} />);
    expect(screen.getByText('0.0%')).toBeTruthy();
  });

  it('shows 50% when half funded', () => {
    render(<FundingProgress current={5_000_000n} target={10_000_000n} />);
    expect(screen.getByText('50.0%')).toBeTruthy();
  });

  it('caps at 100% when overfunded', () => {
    render(<FundingProgress current={20_000_000n} target={10_000_000n} />);
    expect(screen.getByText('100.0%')).toBeTruthy();
  });

  it('handles zero target gracefully without throwing', () => {
    expect(() => {
      render(<FundingProgress current={0n} target={0n} />);
    }).not.toThrow();
  });
});
