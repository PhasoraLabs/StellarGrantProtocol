import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { GrantCard } from '@/components/grants/GrantCard';

vi.mock('@/lib/tokens', () => ({
  getTokenMetadata: vi.fn().mockResolvedValue({ symbol: 'XLM', decimals: 7 }),
  formatTokenAmount: vi.fn(
    (amount: bigint, _decimals: number, opts?: { symbol?: string; showSymbol?: boolean }) =>
      `${Number(amount) / 1e7}${opts?.showSymbol ? ` ${opts.symbol ?? 'XLM'}` : ''}`,
  ),
}));

const GRANT = {
  id: 1,
  title: 'Test Grant Alpha',
  status: 1,
  funded: 5_000_000n,
  budget: 10_000_000n,
  deadline: BigInt(Math.floor(Date.now() / 1000) + 86400),
};

describe('GrantCard', () => {
  it('renders the grant title', () => {
    render(<GrantCard grant={GRANT} />);
    expect(screen.getByText('Test Grant Alpha')).toBeTruthy();
  });

  it('renders the status badge', () => {
    render(<GrantCard grant={GRANT} />);
    expect(screen.getByText('Active')).toBeTruthy();
  });

  it('calls onClick when clicked', () => {
    const handler = vi.fn();
    render(<GrantCard grant={GRANT} onClick={handler} />);
    const card = screen.getByText('Test Grant Alpha').closest('div[class]') as HTMLElement;
    card.click();
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('does not render FundingProgress in compact mode', () => {
    render(<GrantCard grant={GRANT} compact />);
    // In compact mode the deadline and target are not shown
    expect(screen.queryByText(/Deadline/i)).toBeNull();
  });
});
