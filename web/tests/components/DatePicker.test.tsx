import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { DatePicker, computeRelativeDateLabel, todayIso } from "@/components/ui/DatePicker";

const NOW = new Date("2026-06-15T12:00:00.000Z");

describe("computeRelativeDateLabel", () => {
  it("returns null for empty / invalid input", () => {
    expect(computeRelativeDateLabel("", NOW)).toBeNull();
    expect(computeRelativeDateLabel("not-a-date", NOW)).toBeNull();
  });

  it("labels a far-future date as muted", () => {
    expect(computeRelativeDateLabel("2026-07-30", NOW)).toEqual({ text: "In 45 days", tone: "muted" });
  });

  it("labels a date within 7 days as a warning", () => {
    expect(computeRelativeDateLabel("2026-06-20", NOW)).toEqual({ text: "In 5 days", tone: "warning" });
  });

  it("labels exactly 7 days ahead as muted", () => {
    expect(computeRelativeDateLabel("2026-06-22", NOW)).toEqual({ text: "In 7 days", tone: "muted" });
  });

  it("labels today as a warning", () => {
    expect(computeRelativeDateLabel("2026-06-15", NOW)).toEqual({ text: "Today", tone: "warning" });
  });

  it("labels a past date as danger", () => {
    expect(computeRelativeDateLabel("2026-06-01", NOW)).toEqual({
      text: "This date is in the past",
      tone: "danger",
    });
  });

  it("uses singular 'day' for 1 day", () => {
    expect(computeRelativeDateLabel("2026-06-16", NOW)).toEqual({ text: "In 1 day", tone: "warning" });
  });
});

describe("todayIso", () => {
  it("formats the date portion", () => {
    expect(todayIso(NOW)).toBe("2026-06-15");
  });
});

describe("DatePicker", () => {
  it("renders the label and value, and emits onChange", () => {
    const onChange = vi.fn();
    render(<DatePicker value="2026-07-30" onChange={onChange} label="Deadline" />);
    expect(screen.getByText("Deadline")).toBeTruthy();
    const input = screen.getByLabelText("Deadline") as HTMLInputElement;
    expect(input.value).toBe("2026-07-30");
    fireEvent.change(input, { target: { value: "2026-08-01" } });
    expect(onChange).toHaveBeenCalledWith("2026-08-01");
  });

  it("shows the relative label for a future date", () => {
    render(<DatePicker value="2026-07-30" onChange={() => {}} />);
    // Far future from "now" — at minimum the "In … days" text renders.
    expect(screen.getByText(/^In \d+ days$/)).toBeTruthy();
  });

  it("shows the error message instead of the relative label", () => {
    render(<DatePicker value="2026-07-30" onChange={() => {}} error="Deadline required" />);
    expect(screen.getByText("Deadline required")).toBeTruthy();
  });
});
