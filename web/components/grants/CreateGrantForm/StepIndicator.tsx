"use client";

/**
 * StepIndicator Component
 * 
 * Renders a horizontal multi-step progress indicator for the Create Grant flow.
 * Steps: 1 Basic Info -> 2 Milestones -> 3 Reviewers -> 4 Review
 */

interface StepIndicatorProps {
  currentStep: number;
  onStepClick: (step: number) => void;
}

export function StepIndicator({ currentStep, onStepClick }: StepIndicatorProps) {
  const steps = [
    { num: 1, label: "Basic Info" },
    { num: 2, label: "Milestones" },
    { num: 3, label: "Reviewers" },
    { num: 4, label: "Review" },
  ];

  return (
    <nav className="w-full border-b border-border-color/40 pb-6 mb-8" aria-label="Progress">
      <ol className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 font-mono text-xs uppercase tracking-wider">
        {steps.map((step, idx) => {
          const isCompleted = step.num < currentStep;
          const isCurrent = step.num === currentStep;

          return (
            <li key={step.num} className="flex-1 w-full flex items-center gap-3">
              <button
                type="button"
                disabled={!isCompleted}
                onClick={() => onStepClick(step.num)}
                className={`flex items-center gap-2 text-left transition-all ${
                  isCompleted
                    ? "text-accent-secondary hover:text-accent-primary cursor-pointer font-bold"
                    : isCurrent
                    ? "text-accent-primary font-bold"
                    : "text-text-muted cursor-not-allowed"
                }`}
              >
                <span
                  className={`flex h-6 w-6 items-center justify-center border font-semibold ${
                    isCompleted
                      ? "border-accent-secondary bg-accent-secondary/10 text-accent-secondary"
                      : isCurrent
                      ? "border-accent-primary ring-1 ring-accent-primary text-accent-primary"
                      : "border-border-color text-text-muted"
                  }`}
                >
                  {step.num}
                </span>
                <span>{step.label}</span>
              </button>

              {idx < steps.length - 1 && (
                <span className="hidden sm:inline text-text-muted/40 font-light grow text-center px-2">
                  →
                </span>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
