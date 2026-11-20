/**
 * BatchBuilder tests (#270)
 */

import { describe, it, expect, vi } from "vitest";
import { BatchBuilder } from "@/lib/stellar/batchBuilder";

describe("BatchBuilder", () => {
  it("starts empty", () => {
    const b = new BatchBuilder();
    expect(b.size).toBe(0);
    expect(b.preview()).toHaveLength(0);
  });

  it("chains add() calls and increments size", () => {
    const b = new BatchBuilder()
      .add("grantFund", { grant_id: "1", amount: 100n })
      .add("milestoneApprove", { grant_id: "1", idx: 0 });
    expect(b.size).toBe(2);
  });

  it("execute() calls executor for each operation", async () => {
    const executor = vi.fn().mockResolvedValue("tx_hash_123");
    const b = new BatchBuilder()
      .add("grantFund", { grant_id: "1", amount: 100n })
      .add("milestoneApprove", { grant_id: "1", idx: 0 });

    await b.execute(executor);

    expect(executor).toHaveBeenCalledTimes(2);
    expect(executor).toHaveBeenCalledWith("grantFund", { grant_id: "1", amount: 100n });
    expect(executor).toHaveBeenCalledWith("milestoneApprove", { grant_id: "1", idx: 0 });
  });

  it("allSucceeded is true when all ops pass", async () => {
    const executor = vi.fn().mockResolvedValue("tx_hash");
    const result = await new BatchBuilder()
      .add("op1", {})
      .add("op2", {})
      .execute(executor);

    expect(result.allSucceeded).toBe(true);
    expect(result.operations.every((o) => o.status === "success")).toBe(true);
  });

  it("allSucceeded is false when any op fails", async () => {
    const executor = vi
      .fn()
      .mockResolvedValueOnce("tx_hash")
      .mockRejectedValueOnce(new Error("contract error"));

    const result = await new BatchBuilder()
      .add("op1", {})
      .add("op2", {})
      .execute(executor);

    expect(result.allSucceeded).toBe(false);
    expect(result.operations[0].status).toBe("success");
    expect(result.operations[1].status).toBe("failed");
    expect(result.operations[1].error).toBe("contract error");
  });

  it("continues executing remaining ops after one fails", async () => {
    const executor = vi
      .fn()
      .mockRejectedValueOnce(new Error("fail"))
      .mockResolvedValueOnce("ok");

    const result = await new BatchBuilder()
      .add("failOp", {})
      .add("okOp", {})
      .execute(executor);

    expect(executor).toHaveBeenCalledTimes(2);
    expect(result.operations[1].status).toBe("success");
  });

  it("returns immediately with allSucceeded when batch is empty", async () => {
    const executor = vi.fn();
    const result = await new BatchBuilder().execute(executor);
    expect(result.allSucceeded).toBe(true);
    expect(executor).not.toHaveBeenCalled();
  });

  it("clear() resets the operation queue", () => {
    const b = new BatchBuilder().add("op1", {}).add("op2", {});
    expect(b.size).toBe(2);
    b.clear();
    expect(b.size).toBe(0);
  });

  it("preview() returns a snapshot without mutating internal state", () => {
    const b = new BatchBuilder().add("op1", {});
    const snap = b.preview();
    expect(snap).toHaveLength(1);
    expect(b.size).toBe(1);
  });
});
