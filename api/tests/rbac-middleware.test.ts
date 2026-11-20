import { describe, expect, it } from "vitest";
import { requireRole } from "../src/middlewares/rbac-middleware";

describe("RBAC role combinations", () => {
  const run = (role: string | undefined, allowed: ("admin" | "reviewer" | "applicant")[]) => {
    const req = { header: () => role } as any;
    const res = { status: () => res, json: () => res } as any;
    let next = false;
    requireRole(...allowed)(req, res, () => { next = true; });
    return next;
  };
  it("allows any explicitly listed role", () => {
    expect(run("reviewer", ["admin", "reviewer"])).toBe(true);
    expect(run("applicant", ["admin", "reviewer"])).toBe(false);
  });
  it("rejects missing roles", () => expect(run(undefined, ["admin"])).toBe(false));
});
