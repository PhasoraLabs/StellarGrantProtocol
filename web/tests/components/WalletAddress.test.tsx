import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { WalletAddress } from '@/components/wallet/WalletAddress';

const FULL_ADDRESS = 'GABCDE1234567890ABCDE1234567890ABCDE1234567890ABCDE1234567890AB';

describe('WalletAddress', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'dispatchEvent', { value: vi.fn(), writable: true });
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
  });

  it('truncates the address to first-6 … last-4 format', () => {
    render(<WalletAddress address={FULL_ADDRESS} />);
    const truncated = `${FULL_ADDRESS.slice(0, 6)}…${FULL_ADDRESS.slice(-4)}`;
    expect(screen.getByText(truncated)).toBeTruthy();
  });

  it('shows the copy button by default', () => {
    render(<WalletAddress address={FULL_ADDRESS} />);
    expect(screen.getByRole('button')).toBeTruthy();
  });

  it('hides the copy button when showCopyIcon is false', () => {
    render(<WalletAddress address={FULL_ADDRESS} showCopyIcon={false} />);
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('renders the avatar circle when showAvatar is true', () => {
    render(<WalletAddress address={FULL_ADDRESS} showAvatar />);
    // The avatar div is aria-hidden; count DOM nodes for the icon
    const { container } = render(<WalletAddress address={FULL_ADDRESS} showAvatar />);
    const avatar = container.querySelector('[aria-hidden="true"].rounded-full');
    expect(avatar).toBeTruthy();
  });

  it('copies to clipboard and dispatches a toast on button click', async () => {
    const dispatch = vi.fn();
    window.dispatchEvent = dispatch;

    render(<WalletAddress address={FULL_ADDRESS} />);
    fireEvent.click(screen.getByRole('button'));

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(FULL_ADDRESS);
      expect(dispatch).toHaveBeenCalled();
    });
  });

  it('does not clip short addresses', () => {
    render(<WalletAddress address="GABC" />);
    expect(screen.getByText('GABC')).toBeTruthy();
  });
});
