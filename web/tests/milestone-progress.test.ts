import { describe, it, expect } from "vitest";
import { getMilestoneProgress } from "@/hooks/useMilestone";

describe("getMilestoneProgress", () => {
  it("counts approved vs total milestones", () => {
    expect(getMilestoneProgress([{ approved: true }, { approved: false }, { approved: true }])).toEqual({
      approved: 2,
      total: 3,
      percent: 67,
    });
  });

  it("returns 0% when there are no milestones", () => {
    expect(getMilestoneProgress([])).toEqual({ approved: 0, total: 0, percent: 0 });
  });
});
