import type { Meta, StoryObj } from "@storybook/react";
import { Pagination } from "@/components/ui/Pagination";

const meta: Meta<typeof Pagination> = {
  title: "UI/Pagination",
  component: Pagination,
  parameters: { backgrounds: { default: "dark" } },
  argTypes: {
    page: { control: { type: "number", min: 1 } },
    totalPages: { control: { type: "number", min: 1 } },
  },
};
export default meta;

type Story = StoryObj<typeof Pagination>;

export const FirstPage: Story = {
  args: { page: 1, totalPages: 10, onPageChange: () => {} },
};

export const MiddlePage: Story = {
  args: { page: 5, totalPages: 10, onPageChange: () => {} },
};

export const LastPage: Story = {
  args: { page: 10, totalPages: 10, onPageChange: () => {} },
};

export const FewPages: Story = {
  args: { page: 2, totalPages: 4, onPageChange: () => {} },
};

export const SinglePage: Story = {
  args: { page: 1, totalPages: 1, onPageChange: () => {} },
};
