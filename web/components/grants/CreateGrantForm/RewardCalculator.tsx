"use client";

import { useState, useCallback } from "react";

interface RewardCalculatorProps {
  totalBudget: number; // in XLM
  milestoneCount: number;
  onDistribute: (rewards: number[]) => void; // called with array of reward amounts
  currentRewards?: number[];
}

export function RewardCalculator({
  totalBudget,
  milestoneCount,
  onDistribute,
  currentRewards = [],
}: RewardCalculatorProps) {
  const [activeMode, setActiveMode] = useState<"equal" | "front-load" | "custom" | null>(null);
  const [weights, setWeights] = useState<number[]>([]);
  const [previousRewards, setPreviousRewards] = useState<number[] | null>(null);
  const [showUndo, setShowUndo] = useState(false);

  // Adjust weights if milestone count changes (syncing state from props during render)
  const [prevMilestoneCount, setPrevMilestoneCount] = useState(milestoneCount);
  if (milestoneCount !== prevMilestoneCount) {
    setPrevMilestoneCount(milestoneCount);
    setWeights(new Array(milestoneCount).fill(1));
  }

  const round = (num: number) => Math.round(num * 100) / 100;

  const distribute = useCallback((mode: "equal" | "front-load" | "custom", customWeights?: number[]) => {
    if (totalBudget <= 0 || milestoneCount <= 0) return;

    // Snapshot current rewards for undo
    setPreviousRewards([...currentRewards]);
    setShowUndo(true);
    setTimeout(() => setShowUndo(false), 5000);

    let newRewards: number[] = [];

    if (mode === "equal") {
      const base = round(totalBudget / milestoneCount);
      newRewards = new Array(milestoneCount).fill(base);
    } else if (mode === "front-load") {
      if (milestoneCount === 1) {
        newRewards = [totalBudget];
      } else if (milestoneCount === 2) {
        newRewards = [round(totalBudget * 0.6), 0];
        newRewards[1] = round(totalBudget - newRewards[0]);
      } else {
        const m1 = round(totalBudget * 0.5);
        const m2 = round(totalBudget * 0.3);
        const remaining = totalBudget - m1 - m2;
        const othersCount = milestoneCount - 2;
        const otherBase = round(remaining / othersCount);
        newRewards = [m1, m2, ...new Array(othersCount).fill(otherBase)];
      }
    } else if (mode === "custom") {
      const currentWeightsArr = customWeights || weights;
      const totalWeight = currentWeightsArr.reduce((sum, w) => sum + w, 0);
      newRewards = currentWeightsArr.map((w) => round((w / totalWeight) * totalBudget));
    }

    // Ensure sum equals totalBudget by adjusting the last milestone
    const currentSum = newRewards.reduce((sum, r) => sum + r, 0);
    const diff = round(totalBudget - currentSum);
    if (newRewards.length > 0) {
      newRewards[newRewards.length - 1] = round(newRewards[newRewards.length - 1] + diff);
    }

    onDistribute(newRewards);
    setActiveMode(mode);
  }, [totalBudget, milestoneCount, weights, currentRewards, onDistribute]);

  const handleUndo = () => {
    if (previousRewards) {
      onDistribute(previousRewards);
      setPreviousRewards(null);
      setShowUndo(false);
      setActiveMode(null);
    }
  };

  const updateWeight = (index: number, value: number) => {
    const newWeights = [...weights];
    newWeights[index] = value;
    setWeights(newWeights);
    if (activeMode === "custom") {
      distribute("custom", newWeights);
    }
  };

  const isDisabled = totalBudget <= 0 || milestoneCount <= 0;

  return (
    <div className="space-y-4 p-4 border border-border-color/30 bg-surface/30 backdrop-blur-sm">
      <div className="flex flex-wrap items-center gap-3">
        <span className="font-mono text-[10px] uppercase tracking-wider text-text-muted mr-2">
          Auto-distribute:
        </span>
        
        <button
          type="button"
          disabled={isDisabled}
          onClick={() => distribute("equal")}
          className={`px-3 py-1.5 font-mono text-xs uppercase tracking-wider border transition-colors ${
            activeMode === "equal"
              ? "bg-accent-primary text-black border-accent-primary font-bold"
              : "border-border-color text-text-muted hover:border-text-primary hover:text-text-primary"
          } disabled:opacity-50 disabled:cursor-not-allowed`}
        >
          ⚖ Split equally
        </button>

        <button
          type="button"
          disabled={isDisabled}
          onClick={() => distribute("front-load")}
          className={`px-3 py-1.5 font-mono text-xs uppercase tracking-wider border transition-colors ${
            activeMode === "front-load"
              ? "bg-accent-primary text-black border-accent-primary font-bold"
              : "border-border-color text-text-muted hover:border-text-primary hover:text-text-primary"
          } disabled:opacity-50 disabled:cursor-not-allowed`}
        >
          ▲ Front-load
        </button>

        <button
          type="button"
          disabled={isDisabled}
          onClick={() => {
            if (activeMode === "custom") {
              setActiveMode(null);
            } else {
              distribute("custom");
            }
          }}
          className={`px-3 py-1.5 font-mono text-xs uppercase tracking-wider border transition-colors ${
            activeMode === "custom"
              ? "bg-accent-primary text-black border-accent-primary font-bold"
              : "border-border-color text-text-muted hover:border-text-primary hover:text-text-primary"
          } disabled:opacity-50 disabled:cursor-not-allowed`}
        >
          🎚 Custom weights
        </button>

        {showUndo && (
          <button
            type="button"
            onClick={handleUndo}
            className="px-3 py-1.5 font-mono text-xs uppercase tracking-wider text-accent-secondary hover:underline underline-offset-4 animate-pulse"
          >
            ↩ Undo
          </button>
        )}
      </div>

      {activeMode === "custom" && (
        <div className="space-y-3 pt-2 border-t border-border-color/10">
          <p className="font-mono text-[10px] text-text-muted uppercase tracking-tight">
            Adjust sliders to weight milestone rewards (1–10)
          </p>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-8 gap-y-4">
            {weights.map((weight, i) => (
              <div key={i} className="space-y-1">
                <div className="flex justify-between font-mono text-[10px]">
                  <span className="text-text-muted">M{i+1} Weight</span>
                  <span className="text-accent-primary">{weight}</span>
                </div>
                <input
                  type="range"
                  min="1"
                  max="10"
                  step="1"
                  value={weight}
                  onChange={(e) => updateWeight(i, parseInt(e.target.value))}
                  className="w-full accent-accent-primary bg-bg-secondary h-1 appearance-none cursor-pointer"
                />
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
