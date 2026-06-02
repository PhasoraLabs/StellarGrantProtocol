import type { Meta, StoryObj } from "@storybook/react";
import { Card } from "@/components/ui/Card";

const meta: Meta<typeof Card> = {
  title: "UI/Card",
  component: Card,
  parameters: { backgrounds: { default: "dark" } },
};
export default meta;

type Story = StoryObj<typeof Card>;

export const Default: Story = {
  args: {
    children: (
      <div style={{ padding: "24px", color: "#E8EDF5", fontFamily: "monospace" }}>
        <h3 style={{ fontFamily: "var(--font-orbitron, sans-serif)", marginBottom: 8 }}>
          Card Title
        </h3>
        <p>Card content goes here.</p>
      </div>
    ),
  },
};

export const WithPadding: Story = {
  args: {
    className: "p-6",
    children: (
      <p style={{ color: "#6B7FA3", fontFamily: "monospace", fontSize: 13 }}>
        This card has padding applied via className.
      </p>
    ),
  },
};
