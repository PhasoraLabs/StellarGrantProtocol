import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConfirmationDialog } from "@/components/ui/ConfirmationDialog";

function setup(props: Partial<React.ComponentProps<typeof ConfirmationDialog>> = {}) {
  const onConfirm = vi.fn();
  const onCancel = vi.fn();
  const utils = render(
    <ConfirmationDialog
      isOpen
      title="Approve this milestone?"
      description="This action cannot be undone."
      onConfirm={onConfirm}
      onCancel={onCancel}
      {...props}
    />,
  );
  return { onConfirm, onCancel, ...utils };
}

describe("ConfirmationDialog", () => {
  it("renders nothing when closed", () => {
    const { container } = render(
      <ConfirmationDialog
        isOpen={false}
        title="t"
        description="d"
        onConfirm={() => {}}
        onCancel={() => {}}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders title, description and default labels", () => {
    setup();
    expect(screen.getByText("Approve this milestone?")).toBeTruthy();
    expect(screen.getByText("This action cannot be undone.")).toBeTruthy();
    expect(screen.getByText("Confirm")).toBeTruthy();
    expect(screen.getByText("Cancel")).toBeTruthy();
  });

  it("uses custom labels and the danger variant", () => {
    const { container } = setup({ variant: "danger", confirmLabel: "Reject", cancelLabel: "Back" });
    expect(screen.getByText("Reject")).toBeTruthy();
    expect(screen.getByText("Back")).toBeTruthy();
    expect(container.querySelector('[data-variant="danger"]')).toBeTruthy();
  });

  it("calls onConfirm / onCancel on the right buttons", () => {
    const { onConfirm, onCancel } = setup();
    fireEvent.click(screen.getByText("Cancel"));
    expect(onCancel).toHaveBeenCalledOnce();
    expect(onConfirm).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText("Confirm"));
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it("disables both buttons and shows a spinner while loading", () => {
    const { onConfirm, onCancel } = setup({ isLoading: true });
    const buttons = screen.getAllByRole("button");
    expect(buttons.every((b) => (b as HTMLButtonElement).disabled)).toBe(true);
    expect(screen.getByRole("status")).toBeTruthy();
    fireEvent.click(buttons[0]);
    fireEvent.click(buttons[1]);
    expect(onConfirm).not.toHaveBeenCalled();
    expect(onCancel).not.toHaveBeenCalled();
  });
});
