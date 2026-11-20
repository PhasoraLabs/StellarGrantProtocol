import type { Meta, StoryObj } from "@storybook/react";
import { WatchButton } from "@/components/grants/WatchButton";

const meta: Meta<typeof WatchButton> = {
  title: "Grants/WatchButton",
  component: WatchButton,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof WatchButton>;

export const Unwatched: Story = {
  args: { grantId: "42" },
};

export const Watched: Story = {
  args: { grantId: "42" },
};
