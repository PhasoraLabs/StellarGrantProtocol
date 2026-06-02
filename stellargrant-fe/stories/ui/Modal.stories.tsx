import type { Meta, StoryObj } from "@storybook/react";
import { ConfirmationDialog } from "@/components/ui/ConfirmationDialog";

const meta: Meta<typeof ConfirmationDialog> = {
  title: "UI/Modal",
  component: ConfirmationDialog,
  parameters: { backgrounds: { default: "dark" }, layout: "fullscreen" },
  argTypes: {
    variant: { control: "radio", options: ["default", "danger"] },
    isLoading: { control: "boolean" },
  },
};
export default meta;

type Story = StoryObj<typeof ConfirmationDialog>;

const baseArgs = {
  isOpen: true,
  onConfirm: () => {},
  onCancel: () => {},
};

export const Default: Story = {
  args: {
    ...baseArgs,
    title: "Confirm Action",
    description: "Are you sure you want to proceed? This action cannot be undone.",
  },
};

export const Danger: Story = {
  args: {
    ...baseArgs,
    variant: "danger",
    title: "Cancel Grant",
    description: "Cancelling will forfeit all funds and cannot be reversed.",
    confirmLabel: "Cancel Grant",
  },
};

export const Loading: Story = {
  args: {
    ...baseArgs,
    title: "Submitting Vote",
    description: "Sending transaction to the Stellar network…",
    isLoading: true,
  },
};

export const BatchVoteConfirm: Story = {
  args: {
    ...baseArgs,
    title: "Confirm Batch Vote",
    description:
      "You are about to approve 3 milestones in a single transaction. This cannot be undone.",
    confirmLabel: "Submit",
  },
};
