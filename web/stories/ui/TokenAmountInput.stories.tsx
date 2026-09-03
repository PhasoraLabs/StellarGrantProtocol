import type { Meta, StoryObj } from "@storybook/react";
import { TokenAmountInput } from "@/components/ui/TokenAmountInput";

const meta: Meta<typeof TokenAmountInput> = {
  title: "UI/TokenAmountInput",
  component: TokenAmountInput,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof TokenAmountInput>;

export const XLM: Story = {
  args: {
    token: "native",
    label: "Amount",
    value: "",
    onChange: () => {},
  },
};

export const USDC: Story = {
  args: {
    token: "USDC",
    label: "Amount",
    value: "",
    onChange: () => {},
  },
};

export const WithError: Story = {
  args: {
    token: "native",
    label: "Amount",
    value: "",
    error: "Amount exceeds available balance",
    onChange: () => {},
  },
};

export const Disabled: Story = {
  args: {
    token: "native",
    label: "Amount",
    value: "",
    disabled: true,
    onChange: () => {},
  },
};
