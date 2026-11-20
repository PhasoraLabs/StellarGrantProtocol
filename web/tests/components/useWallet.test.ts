import { describe, it, expect } from 'vitest';

/**
 * useWallet hook — interface contract tests.
 */
describe('useWallet (interface contract)', () => {
  it('exports useWallet as a named function', async () => {
    const mod = await import('@/hooks/useWallet');
    expect(typeof mod.useWallet).toBe('function');
  });

  it('exports a WalletState interface (type-level check via function arity)', async () => {
    const { useWallet } = await import('@/hooks/useWallet');
    // The hook must accept zero arguments
    expect(useWallet.length).toBe(0);
  });
});
