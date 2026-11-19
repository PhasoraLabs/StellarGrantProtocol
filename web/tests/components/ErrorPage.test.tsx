import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import ErrorPage from '@/app/error';

vi.mock('@/components/landing/StarField', () => ({ StarField: () => null }));

describe('app/error.tsx', () => {
  it('calls reset() when the retry button is clicked', () => {
    const reset = vi.fn();
    vi.spyOn(console, 'error').mockImplementation(() => {});

    render(<ErrorPage error={new Error('boom')} reset={reset} />);
    fireEvent.click(screen.getByRole('button', { name: /try again/i }));

    expect(reset).toHaveBeenCalledTimes(1);
  });
});
