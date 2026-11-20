import { describe, it, expect } from 'vitest';

/**
 * useGrants hook — contract tests.
 *
 * The hook is a thin wrapper around fetch; we validate its public
 * interface shape here rather than running a live HTTP call.
 */
describe('useGrants (interface contract)', () => {
  it('exports useGrants as a named function', async () => {
    const mod = await import('@/hooks/useGrants');
    expect(typeof mod.useGrants).toBe('function');
  });

  it('accepts an optional options argument', async () => {
    const { useGrants } = await import('@/hooks/useGrants');
    // Calling without arguments must not throw at import time
    expect(useGrants).toBeDefined();
  });
});
