"use client";

import { useEffect, useRef } from "react";
import { FormProvider, useForm } from "react-hook-form";
import { Step2Milestones } from "./Step2Milestones";
import { TotalBudgetField } from "./TotalBudgetField";
import { defaultCreateGrantValues, type CreateGrantFormValues } from "./types";
import { useGrantDraft } from "@/hooks/useGrantDraft";

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
  const methods = useForm<CreateGrantFormValues>({
    defaultValues: defaultCreateGrantValues,
    mode: "onChange",
  });
  const { draft, saveDraft, clearDraft, hasDraft, draftAge } = useGrantDraft();
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const watchedValues = methods.watch();

  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      saveDraft(watchedValues as Partial<CreateGrantFormValues>);
    }, 2000);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [watchedValues, saveDraft]);

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
    methods.reset({
      ...defaultCreateGrantValues,
      ...draft,
    });
  };

  const handleDiscard = () => {
    clearDraft();
  };

  return (
    <FormProvider {...methods}>
      {hasDraft && (
        <DraftRestoreBanner
          draftAge={draftAge}
          onRestore={handleRestore}
          onDiscard={handleDiscard}
        />
      )}
      <form className="max-w-2xl space-y-8 mt-6">
        <TotalBudgetField />
        <Step2Milestones />
      </form>
    </FormProvider>
  );
}

export { Step2Milestones } from "./Step2Milestones";
export { BudgetDistributionChart } from "./BudgetDistributionChart";
