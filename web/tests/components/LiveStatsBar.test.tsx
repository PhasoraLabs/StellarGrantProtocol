import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { LiveStatsBar } from '@/components/landing/LiveStatsBar';

describe('LiveStatsBar', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('hides the bar and shows no fabricated numbers when the API fails', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('down')));
    const { container } = render(<LiveStatsBar />);
    await waitFor(() => expect(container.firstChild).toBeNull());
    expect(screen.queryByText('Grants Created')).toBeNull();
  });
});
