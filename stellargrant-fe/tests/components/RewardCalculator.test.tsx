/**
 * RewardCalculator Tests
 *
 * Tests for the RewardCalculator component and its distribution logic.
 *
 * @see https://github.com/StellarGrant/StellarGrant-fe/issues/391
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { RewardCalculator } from "@/components/grants/CreateGrantForm/RewardCalculator";

describe("RewardCalculator", () => {
  const defaultProps = {
    totalBudget: 1000,
    milestoneCount: 3,
    onDistribute: vi.fn(),
  };

  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  // ── Disabled states ──────────────────────────────────────────────────────

  it("disables all buttons when budget is 0", () => {
    render(<RewardCalculator {...defaultProps} totalBudget={0} />);

    const buttons = screen.getAllByRole("button");
    buttons.forEach((btn) => {
      if (btn.textContent?.includes("Split") || btn.textContent?.includes("Front") || btn.textContent?.includes("Custom")) {
        expect(btn).toBeDisabled();
      }
    });
  });

  it("disables all buttons when milestone count is 0", () => {
    render(<RewardCalculator {...defaultProps} milestoneCount={0} />);

    const buttons = screen.getAllByRole("button");
    buttons.forEach((btn) => {
      if (btn.textContent?.includes("Split") || btn.textContent?.includes("Front") || btn.textContent?.includes("Custom")) {
        expect(btn).toBeDisabled();
      }
    });
  });

  it("shows help message when disabled", () => {
    render(<RewardCalculator {...defaultProps} totalBudget={0} />);

    expect(screen.getByText(/Set a budget and add milestones/)).toBeDefined();
  });

  // ── Equal split ──────────────────────────────────────────────────────────

  it("splits budget equally across milestones", () => {
    const onDistribute = vi.fn();
    render(<RewardCalculator {...defaultProps} onDistribute={onDistribute} />);

    // Click "Split equally" mode button
    fireEvent.click(screen.getByText("⚖ Split equally"));

    // Click "Apply Equal Split"
    fireEvent.click(screen.getByText("Apply Equal Split"));

    expect(onDistribute).toHaveBeenCalledWith([333.34, 333.33, 333.33]);
  });

  it("handles single milestone equal split", () => {
    const onDistribute = vi.fn();
    render(
      <RewardCalculator
        {...defaultProps}
        milestoneCount={1}
        onDistribute={onDistribute}
      />
    );

    fireEvent.click(screen.getByText("⚖ Split equally"));
    fireEvent.click(screen.getByText("Apply Equal Split"));

    expect(onDistribute).toHaveBeenCalledWith([1000]);
  });

  it("ensures sum equals totalBudget after equal split", () => {
    const onDistribute = vi.fn();
    render(
      <RewardCalculator
        {...defaultProps}
        totalBudget={100}
        milestoneCount={3}
        onDistribute={onDistribute}
      />
    );

    fireEvent.click(screen.getByText("⚖ Split equally"));
    fireEvent.click(screen.getByText("Apply Equal Split"));

    const rewards = onDistribute.mock.calls[0][0];
    const sum = rewards.reduce((a: number, b: number) => a + b, 0);
    expect(sum).toBe(100);
  });

  // ── Front-loaded ─────────────────────────────────────────────────────────

  it("distributes front-loaded: 50/30/20 for 3 milestones", () => {
    const onDistribute = vi.fn();
    render(
      <RewardCalculator
        {...defaultProps}
        totalBudget={1000}
        milestoneCount={3}
        onDistribute={onDistribute}
      />
    );

    fireEvent.click(screen.getByText("▲ Front-load"));
    fireEvent.click(screen.getByText("Apply Front-load"));

    expect(onDistribute).toHaveBeenCalledWith([500, 300, 200]);
  });

  it("handles single milestone front-load", () => {
    const onDistribute = vi.fn();
    render(
      <RewardCalculator
        {...defaultProps}
        milestoneCount={1}
        onDistribute={onDistribute}
      />
    );

    fireEvent.click(screen.getByText("▲ Front-load"));
    fireEvent.click(screen.getByText("Apply Front-load"));

    expect(onDistribute).toHaveBeenCalledWith([1000]);
  });

  it("handles two milestones front-load", () => {
    const onDistribute = vi.fn();
    render(
      <RewardCalculator
        {...defaultProps}
        milestoneCount={2}
        onDistribute={onDistribute}
      />
    );

    fireEvent.click(screen.getByText("▲ Front-load"));
    fireEvent.click(screen.getByText("Apply Front-load"));

    expect(onDistribute).toHaveBeenCalledWith([500, 500]);
  });

  // ── Custom weights ───────────────────────────────────────────────────────

  it("distributes by custom weights", () => {
    const onDistribute = vi.fn();
    render(
      <RewardCalculator
        {...defaultProps}
        totalBudget={1000}
        milestoneCount={3}
        onDistribute={onDistribute}
      />
    );

    // Switch to custom mode
    fireEvent.click(screen.getByText("⚙ Custom weights"));

    // Default weights are all 5, so equal distribution
    fireEvent.click(screen.getByText("Apply Custom Weights"));

    const rewards = onDistribute.mock.calls[0][0];
    const sum = rewards.reduce((a: number, b: number) => a + b, 0);
    expect(sum).toBe(1000);
  });

  // ── Undo ─────────────────────────────────────────────────────────────────

  it("shows undo button after distribution", () => {
    render(<RewardCalculator {...defaultProps} />);

    fireEvent.click(screen.getByText("⚖ Split equally"));
    fireEvent.click(screen.getByText("Apply Equal Split"));

    expect(screen.getByText("↩ Undo")).toBeDefined();
  });

  it("hides undo button after 5 seconds", () => {
    render(<RewardCalculator {...defaultProps} />);

    fireEvent.click(screen.getByText("⚖ Split equally"));
    fireEvent.click(screen.getByText("Apply Equal Split"));

    expect(screen.getByText("↩ Undo")).toBeDefined();

    act(() => {
      vi.advanceTimersByTime(5000);
    });

    expect(screen.queryByText("↩ Undo")).toBeNull();
  });

  it("undo reverts to previous values", () => {
    const onDistribute = vi.fn();
    render(<RewardCalculator {...defaultProps} onDistribute={onDistribute} />);

    // First distribution
    fireEvent.click(screen.getByText("⚖ Split equally"));
    fireEvent.click(screen.getByText("Apply Equal Split"));

    const firstCall = onDistribute.mock.calls[0][0];

    // Click undo
    fireEvent.click(screen.getByText("↩ Undo"));

    expect(onDistribute).toHaveBeenCalledTimes(2);
    // Undo should call with the same values (the "previous" state)
    expect(onDistribute.mock.calls[1][0]).toEqual(firstCall);
  });

  // ── Edge cases ───────────────────────────────────────────────────────────

  it("handles budget with many decimal places", () => {
    const onDistribute = vi.fn();
    render(
      <RewardCalculator
        {...defaultProps}
        totalBudget={99.99}
        milestoneCount={3}
        onDistribute={onDistribute}
      />
    );

    fireEvent.click(screen.getByText("⚖ Split equally"));
    fireEvent.click(screen.getByText("Apply Equal Split"));

    const rewards = onDistribute.mock.calls[0][0];
    const sum = rewards.reduce((a: number, b: number) => a + b, 0);
    expect(sum).toBe(99.99);
  });

  it("rounds rewards to 2 decimal places", () => {
    const onDistribute = vi.fn();
    render(
      <RewardCalculator
        {...defaultProps}
        totalBudget={100}
        milestoneCount={3}
        onDistribute={onDistribute}
      />
    );

    fireEvent.click(screen.getByText("⚖ Split equally"));
    fireEvent.click(screen.getByText("Apply Equal Split"));

    const rewards = onDistribute.mock.calls[0][0];
    rewards.forEach((r: number) => {
      const decimals = r.toString().split(".")[1]?.length ?? 0;
      expect(decimals).toBeLessThanOrEqual(2);
    });
  });
});
