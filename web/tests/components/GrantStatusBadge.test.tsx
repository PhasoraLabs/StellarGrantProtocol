import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { GrantStatusBadge } from '@/components/grants/GrantStatusBadge';

describe('GrantStatusBadge', () => {
  it('renders Pending for status 0', () => {
    render(<GrantStatusBadge status={0} />);
    expect(screen.getByText('Pending')).toBeTruthy();
  });

  it('renders Active for status 1', () => {
    render(<GrantStatusBadge status={1} />);
    expect(screen.getByText('Active')).toBeTruthy();
  });

  it('renders Completed for status 3', () => {
    render(<GrantStatusBadge status={3} />);
    expect(screen.getByText('Completed')).toBeTruthy();
  });

  it('renders Cancelled for status 4', () => {
    render(<GrantStatusBadge status={4} />);
    expect(screen.getByText('Cancelled')).toBeTruthy();
  });

  it('falls back to Pending for unknown status', () => {
    render(<GrantStatusBadge status={99} />);
    expect(screen.getByText('Pending')).toBeTruthy();
  });
});
