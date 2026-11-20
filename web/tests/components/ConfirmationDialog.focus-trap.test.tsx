import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConfirmationDialog } from "@/components/ui/ConfirmationDialog";

function dialog(isOpen: boolean) {
  return (
    <ConfirmationDialog
      isOpen={isOpen}
      title="Confirm?"
      description="d"
      onConfirm={() => {}}
      onCancel={() => {}}
    />
  );
}

describe("ConfirmationDialog focus trap", () => {
  it("keeps Tab inside the dialog and restores focus on close", () => {
    const trigger = document.createElement("button");
    document.body.appendChild(trigger);
    trigger.focus();

    const { rerender } = render(dialog(true));
    const cancel = screen.getByRole("button", { name: "Cancel" });
    const confirm = screen.getByRole("button", { name: "Confirm" });
    expect(document.activeElement).toBe(cancel);

    confirm.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(document.activeElement).toBe(cancel);

    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(confirm);

    rerender(dialog(false));
    expect(document.activeElement).toBe(trigger);
    trigger.remove();
  });
});
