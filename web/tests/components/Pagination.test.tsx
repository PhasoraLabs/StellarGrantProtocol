import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Pagination } from '@/components/ui/Pagination';

describe('Pagination', () => {
  it('renders nothing when totalPages is 1', () => {
    const { container } = render(
      <Pagination page={1} totalPages={1} onPageChange={() => undefined} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('shows prev/next buttons', () => {
    render(<Pagination page={2} totalPages={5} onPageChange={() => undefined} />);
    expect(screen.getByLabelText('Previous page')).toBeTruthy();
    expect(screen.getByLabelText('Next page')).toBeTruthy();
  });

  it('disables Prev on the first page', () => {
    render(<Pagination page={1} totalPages={5} onPageChange={() => undefined} />);
    const prev = screen.getByLabelText('Previous page') as HTMLButtonElement;
    expect(prev.disabled).toBe(true);
  });

  it('disables Next on the last page', () => {
    render(<Pagination page={5} totalPages={5} onPageChange={() => undefined} />);
    const next = screen.getByLabelText('Next page') as HTMLButtonElement;
    expect(next.disabled).toBe(true);
  });

  it('calls onPageChange with the correct page when Next is clicked', () => {
    const handler = vi.fn();
    render(<Pagination page={2} totalPages={5} onPageChange={handler} />);
    fireEvent.click(screen.getByLabelText('Next page'));
    expect(handler).toHaveBeenCalledWith(3);
  });

  it('calls onPageChange with the correct page when Prev is clicked', () => {
    const handler = vi.fn();
    render(<Pagination page={3} totalPages={5} onPageChange={handler} />);
    fireEvent.click(screen.getByLabelText('Previous page'));
    expect(handler).toHaveBeenCalledWith(2);
  });

  it('marks the active page with aria-current="page"', () => {
    render(<Pagination page={2} totalPages={5} onPageChange={() => undefined} />);
    const active = screen.getByLabelText('Page 2');
    expect(active.getAttribute('aria-current')).toBe('page');
  });
});
