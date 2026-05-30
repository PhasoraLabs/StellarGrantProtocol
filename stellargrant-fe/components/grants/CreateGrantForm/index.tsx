"use client";

import { useEffect, useRef, useState } from "react";
import { FormProvider, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { StepIndicator } from "./StepIndicator";
import { Step1BasicInfo } from "./Step1BasicInfo";
import { Step2Milestones } from "./Step2Milestones";
import { Step3Reviewers } from "./Step3Reviewers";
import { Step4ReviewSubmit } from "./Step4ReviewSubmit";
import { fullGrantSchema, type GrantFormData } from "@/lib/schemas/grant";
import { useGrantDraft } from "@/hooks/useGrantDraft";

const defaultValues: GrantFormData = {
  title: "",
  description: "",
  recipientAddress: "",
  totalBudget: 100,
  budgetToken: "native",
  deadline: "",
  milestones: [{ title: "Milestone 1", description: "", reward: 100 }],
  reviewers: ["", "", ""],
  quorum: 2,
};

function DraftRestoreBanner({
  draftAge,
  onRestore,
  onDiscard,
}: {
  draftAge: string;
  onRestore: () => void;
  onDiscard: () => void;
}) {
  return (
    <div className="border border-warning/40 bg-warning/10 p-4 flex items-center justify-between gap-4">
      <p className="font-mono text-sm text-text-primary">
        You have a saved draft from {draftAge}.
      </p>
      <div className="flex items-center gap-2 shrink-0">
        <button
          type="button"
          onClick={onRestore}
          className="px-3 py-1.5 font-mono text-xs uppercase tracking-wider border border-accent-secondary text-accent-secondary hover:bg-accent-secondary/10 transition-colors"
        >
          Restore Draft
        </button>
        <button
          type="button"
          onClick={onDiscard}
          className="px-3 py-1.5 font-mono text-xs uppercase tracking-wider text-text-muted hover:text-danger transition-colors"
        >
          Discard Draft
        </button>
      </div>
    </div>
  );
}

export function CreateGrantForm() {
  const [currentStep, setCurrentStep] = useState(1);
  const methods = useForm<GrantFormData>({
    resolver: zodResolver(fullGrantSchema),
    defaultValues,
    mode: "onChange",
  });

  const { draft, saveDraft, clearDraft, hasDraft, draftAge } = useGrantDraft();
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const watchedValues = methods.watch();

  // Debounced save draft
  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      saveDraft({ ...watchedValues, currentStep } as Partial<GrantFormData> & { currentStep?: number });
    }, 2000);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [watchedValues, currentStep, saveDraft]);

  // Prevent accidental exits
  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = "";
    };

    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, []);

  const handleRestore = () => {
    if (!draft) return;
    const { currentStep: draftStep, ...draftData } = draft;
    methods.reset({
      ...defaultValues,
      ...draftData,
    });
    if (draftStep) {
      setCurrentStep(draftStep);
    }
  };

  const handleDiscard = () => {
    clearDraft();
  };

  const stepFields: Record<number, Array<keyof GrantFormData>> = {
    1: ["title", "description", "recipientAddress", "totalBudget", "budgetToken", "deadline"],
    2: ["milestones"],
    3: ["reviewers", "quorum"],
  };

  const handleContinue = async () => {
    const fieldsToValidate = stepFields[currentStep];
    if (fieldsToValidate) {
      const isValid = await methods.trigger(fieldsToValidate);
      if (!isValid) return;

      // Manual checks for cross-field rules to ensure strict visual feedback
      if (currentStep === 2) {
        const milestones = methods.getValues("milestones") || [];
        const totalBudget = methods.getValues("totalBudget") || 0;
        const totalAllocated = milestones.reduce(
          (sum, m) => sum + (Number(m?.reward) || 0),
          0
        );
        const isBalanced = Math.abs(totalAllocated - totalBudget) < 0.0001;
        if (!isBalanced) return;
      }

      if (currentStep === 3) {
        const quorum = methods.getValues("quorum") || 0;
        const reviewers = methods.getValues("reviewers") || [];
        if (quorum > reviewers.length) return;
      }
    }

    setCurrentStep((prev) => Math.min(prev + 1, 4));
  };

  const handleBack = () => {
    setCurrentStep((prev) => Math.max(prev - 1, 1));
  };

  const handleStepIndicatorClick = (step: number) => {
    if (step < currentStep) {
      setCurrentStep(step);
    }
  };

  const renderActiveStep = () => {
    switch (currentStep) {
      case 1:
        return <Step1BasicInfo />;
      case 2:
        return <Step2Milestones />;
      case 3:
        return <Step3Reviewers />;
      case 4:
        return <Step4ReviewSubmit />;
      default:
        return <Step1BasicInfo />;
    }
  };

  return (
    <FormProvider {...methods}>
      <div className="max-w-3xl mx-auto space-y-6">
        {hasDraft && (
          <DraftRestoreBanner
            draftAge={draftAge}
            onRestore={handleRestore}
            onDiscard={handleDiscard}
          />
        )}

        <StepIndicator
          currentStep={currentStep}
          onStepClick={handleStepIndicatorClick}
        />

        <div className="bg-bg-secondary/40 border border-border-color/30 p-6 backdrop-blur-sm shadow-lg">
          {renderActiveStep()}
        </div>

        {/* Navigation Bar */}
        {currentStep < 4 && (
          <div className="flex justify-between items-center pt-4 border-t border-border-color/20">
            {currentStep > 1 ? (
              <button
                type="button"
                onClick={handleBack}
                className="px-6 py-2.5 font-mono text-xs uppercase tracking-widest border border-border-color hover:border-text-primary text-text-muted hover:text-text-primary transition-all duration-200"
              >
                ← Back
              </button>
            ) : (
              <div />
            )}

            <button
              type="button"
              onClick={handleContinue}
              className="px-6 py-2.5 font-mono text-xs uppercase tracking-widest bg-accent-primary hover:bg-accent-primary/80 text-black font-bold transition-all duration-200"
            >
              Continue →
            </button>
          </div>
        )}
      </div>
    </FormProvider>
  );
}

export { Step2Milestones } from "./Step2Milestones";
export { BudgetDistributionChart } from "./BudgetDistributionChart";
