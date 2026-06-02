import type { Meta, StoryObj } from "@storybook/react";
import { SearchInput } from "@/components/ui/SearchInput";

const meta: Meta<typeof SearchInput> = {
  title: "UI/Input",
  component: SearchInput,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof SearchInput>;

export const Default: Story = {
  args: { placeholder: "Search grants…" },
};

export const WithValue: Story = {
  args: { placeholder: "Search grants…", defaultValue: "AI Safety" },
};

export const Disabled: Story = {
  args: { placeholder: "Unavailable", disabled: true },
};
