"use client";

/**
 * RewardCalculator Component
 *
 * Provides automatic reward distribution across milestones.
 * Supports three modes:
 *   1. Equal split — divides budget evenly
 *   2. Front-loaded — 50/30/remainder pattern
 *   3. Custom weight — user-defined slider weights
 *
 * Includes an Undo button that reverts to previous values for 5 seconds.
 *
 * @see https://github.com/StellarGrant/StellarGrant-fe/issues/391
 */

import { useState, useCallback, useRef, useEffect } from "react";

// ── Types ────────────────────────────────────────────────────────────────────

interface RewardCalculatorProps {
  /** Total budget in XLM */
  totalBudget: number;
  /** Number of milestones to distribute across */
  milestoneCount: number;
  /** Called with array of reward amounts (in XLM) when distribution is applied */
  onDistribute: (rewards: number[]) => void;
}

type DistributionMode = "equal" | "frontload" | "custom";

// ── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Round to 2 decimal places and ensure sum equals totalBudget.
 * Adjusts the last element for rounding errors.
 */
function roundAndBalance(values: number[], totalBudget: number): number[] {
  const rounded = values.map((v) => Math.round(v * 100) / 100);
  const sum = rounded.reduce((a, b) => a + b, 0);
  const diff = Math.round((totalBudget - sum) * 100) / 100;

  // Distribute rounding error to the last non-zero element
  if (diff !== 0 && rounded.length > 0) {
    rounded[rounded.length - 1] = Math.round((rounded[rounded.length - 1] + diff) * 100) / 100;
  }

  return rounded;
}

/**
 * Equal split: divide budget evenly across milestones.
 * Last milestone gets any rounding remainder.
 */
function equalSplit(totalBudget: number, milestoneCount: number): number[] {
  if (milestoneCount === 0) return [];
  const base = totalBudget / milestoneCount;
  const rewards = new Array(milestoneCount).fill(base);
  return roundAndBalance(rewards, totalBudget);
}

/**
 * Front-loaded distribution: 50% first, 30% second, remainder split among rest.
 * For N milestones: first gets 50%, second gets 30%, rest share remaining 20%.
 */
function frontLoad(totalBudget: number, milestoneCount: number): number[] {
  if (milestoneCount === 0) return [];
  if (milestoneCount === 1) return [totalBudget];

  const first = totalBudget * 0.5;
  const second = totalBudget * 0.3;
  const remainder = totalBudget - first - second;
  const restCount = milestoneCount - 2;

  const rewards: number[] = [first, second];

  if (restCount > 0) {
    const perRest = remainder / restCount;
    for (let i = 0; i < restCount; i++) {
      rewards.push(perRest);
    }
  }

  return roundAndBalance(rewards, totalBudget);
}

/**
 * Custom weight distribution: rewards proportional to weights.
 */
function customWeight(totalBudget: number, weights: number[]): number[] {
  const totalWeight = weights.reduce((a, b) => a + b, 0);
  if (totalWeight === 0) return weights.map(() => 0);

  const rewards = weights.map((w) => (w / totalWeight) * totalBudget);
  return roundAndBalance(rewards, totalBudget);
}

// ── Component ────────────────────────────────────────────────────────────────

export function RewardCalculator({
  totalBudget,
  milestoneCount,
  onDistribute,
}: RewardCalculatorProps) {
  const [mode, setMode] = useState<DistributionMode>("equal");
  const [weights, setWeights] = useState<number[]>([]);
  const [previousRewards, setPreviousRewards] = useState<number[] | null>(null);
  const [showUndo, setShowUndo] = useState(false);
  const undoTimerRef = useRef<NodeJS.Timeout | null>(null);

  // Initialize weights when milestone count changes
  useEffect(() => {
    setWeights((prev) => {
      if (prev.length === milestoneCount) return prev;
      const newWeights = [];
      for (let i = 0; i < milestoneCount; i++) {
        newWeights.push(prev[i] ?? 5); // default weight: 5
      }
      return newWeights;
    });
  }, [milestoneCount]);

  // Cleanup undo timer on unmount
  useEffect(() => {
    return () => {
      if (undoTimerRef.current) {
        clearTimeout(undoTimerRef.current);
      }
    };
  }, []);

  const isDisabled = totalBudget === 0 || milestoneCount === 0;

  /**
   * Apply distribution and notify parent.
   */
  const applyDistribution = useCallback(
    (rewards: number[]) => {
      // Store current rewards for undo
      setPreviousRewards(rewards);
      setShowUndo(true);

      // Clear any existing timer
      if (undoTimerRef.current) {
        clearTimeout(undoTimerRef.current);
      }

      // Hide undo button after 5 seconds
      undoTimerRef.current = setTimeout(() => {
        setShowUndo(false);
        setPreviousRewards(null);
      }, 5000);

      onDistribute(rewards);
    },
    [onDistribute]
  );

  /**
   * Undo the last distribution.
   */
  const handleUndo = useCallback(() => {
    if (previousRewards) {
      onDistribute(previousRewards);
      setShowUndo(false);
      setPreviousRewards(null);
      if (undoTimerRef.current) {
        clearTimeout(undoTimerRef.current);
      }
    }
  }, [previousRewards, onDistribute]);

  /**
   * Handle equal split click.
   */
  const handleEqualSplit = useCallback(() => {
    const rewards = equalSplit(totalBudget, milestoneCount);
    applyDistribution(rewards);
  }, [totalBudget, milestoneCount, applyDistribution]);

  /**
   * Handle front-load click.
   */
  const handleFrontLoad = useCallback(() => {
    const rewards = frontLoad(totalBudget, milestoneCount);
    applyDistribution(rewards);
  }, [totalBudget, milestoneCount, applyDistribution]);

  /**
   * Handle custom weight apply.
   */
  const handleCustomWeight = useCallback(() => {
    const rewards = customWeight(totalBudget, weights);
    applyDistribution(rewards);
  }, [totalBudget, weights, applyDistribution]);

  /**
   * Update a single weight value.
   */
  const updateWeight = useCallback((index: number, value: number) => {
    setWeights((prev) => {
      const next = [...prev];
      next[index] = Math.max(1, Math.min(10, value)); // clamp 1-10
      return next;
    });
  }, []);

  return (
    <div
      className="rounded-sm border p-4"
      style={{ background: "#0D1829", borderColor: "#1E3A5F" }}
    >
      <h3 className="text-sm font-semibold text-text-secondary mb-3 uppercase tracking-wider">
        Reward Distribution
      </h3>

      {/* Mode selector buttons */}
      <div className="flex flex-wrap gap-2 mb-4">
        <button
          onClick={() => setMode("equal")}
          disabled={isDisabled}
          className={`px-3 py-1.5 text-xs font-medium rounded-sm border transition-colors ${
            mode === "equal"
              ? "bg-accent-secondary text-background border-accent-secondary"
              : "border-accent-secondary/40 text-accent-secondary hover:bg-accent-secondary/10"
          } ${isDisabled ? "opacity-50 cursor-not-allowed" : ""}`}
        >
          ⚖ Split equally
        </button>
        <button
          onClick={() => setMode("frontload")}
          disabled={isDisabled}
          className={`px-3 py-1.5 text-xs font-medium rounded-sm border transition-colors ${
            mode === "frontload"
              ? "bg-accent-secondary text-background border-accent-secondary"
              : "border-accent-secondary/40 text-accent-secondary hover:bg-accent-secondary/10"
          } ${isDisabled ? "opacity-50 cursor-not-allowed" : ""}`}
        >
          ▲ Front-load
        </button>
        <button
          onClick={() => setMode("custom")}
          disabled={isDisabled}
          className={`px-3 py-1.5 text-xs font-medium rounded-sm border transition-colors ${
            mode === "custom"
              ? "bg-accent-secondary text-background border-accent-secondary"
              : "border-accent-secondary/40 text-accent-secondary hover:bg-accent-secondary/10"
          } ${isDisabled ? "opacity-50 cursor-not-allowed" : ""}`}
        >
          ⚙ Custom weights
        </button>
      </div>

      {/* Description for selected mode */}
      <p className="text-xs text-text-muted mb-3">
        {mode === "equal" &&
          `Divides ${totalBudget} XLM evenly across ${milestoneCount} milestones.`}
        {mode === "frontload" &&
          `50% to first milestone, 30% to second, remainder split among rest.`}
        {mode === "custom" &&
          `Adjust sliders (1–10) to set relative weight for each milestone.`}
      </p>

      {/* Custom weight sliders */}
      {mode === "custom" && milestoneCount > 0 && (
        <div className="space-y-2 mb-4">
          {Array.from({ length: milestoneCount }, (_, i) => (
            <div key={i} className="flex items-center gap-3">
              <span className="text-xs text-text-muted w-24 shrink-0">
                Milestone {i + 1}
              </span>
              <input
                type="range"
                min={1}
                max={10}
                step={1}
                value={weights[i] ?? 5}
                onChange={(e) => updateWeight(i, parseInt(e.target.value, 10))}
                className="flex-1 h-1 accent-accent-secondary"
                disabled={isDisabled}
              />
              <span className="text-xs font-mono text-accent-secondary w-6 text-right">
                {weights[i] ?? 5}
              </span>
            </div>
          ))}
        </div>
      )}

      {/* Apply button */}
      <div className="flex items-center gap-3">
        {mode === "equal" && (
          <button
            onClick={handleEqualSplit}
            disabled={isDisabled}
            className="px-4 py-2 text-xs font-medium rounded-sm bg-accent-secondary text-background hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Apply Equal Split
          </button>
        )}
        {mode === "frontload" && (
          <button
            onClick={handleFrontLoad}
            disabled={isDisabled}
            className="px-4 py-2 text-xs font-medium rounded-sm bg-accent-secondary text-background hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Apply Front-load
          </button>
        )}
        {mode === "custom" && (
          <button
            onClick={handleCustomWeight}
            disabled={isDisabled}
            className="px-4 py-2 text-xs font-medium rounded-sm bg-accent-secondary text-background hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Apply Custom Weights
          </button>
        )}

        {/* Undo button */}
        {showUndo && (
          <button
            onClick={handleUndo}
            className="px-4 py-2 text-xs font-medium rounded-sm border border-text-muted text-text-muted hover:bg-text-muted/10 transition-colors"
          >
            ↩ Undo
          </button>
        )}
      </div>

      {/* Disabled state message */}
      {isDisabled && (
        <p className="text-xs text-text-muted mt-2 italic">
          Set a budget and add milestones to enable distribution.
        </p>
      )}
    </div>
  );
}
