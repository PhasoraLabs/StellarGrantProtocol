import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SearchInput } from "@/components/ui/SearchInput";

describe("SearchInput", () => {
  it("renders the value and calls onChange", () => {
    const onChange = vi.fn();
    render(<SearchInput value="stellar" onChange={onChange} />);
    const input = screen.getByRole("searchbox");
    fireEvent.change(input, { target: { value: "infra" } });
    expect(onChange).toHaveBeenCalledWith("infra");
  });

  it("shows a clear button when value is non-empty", () => {
    const onChange = vi.fn();
    render(<SearchInput value="query" onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: /clear search/i }));
    expect(onChange).toHaveBeenCalledWith("");
  });
});
