import { describe, it, expect } from "vitest";
import { formatUnreadBadge } from "@/hooks/useNotifications";

describe("formatUnreadBadge", () => {
  it("shows the count up to 99 and caps at 99+", () => {
    expect(formatUnreadBadge(1)).toBe("1");
    expect(formatUnreadBadge(99)).toBe("99");
    expect(formatUnreadBadge(100)).toBe("99+");
  });
});
