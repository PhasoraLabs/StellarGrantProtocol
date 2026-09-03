/**
 * Error Mapping Tests — Issue #250
 *
 * Covers:
 *   errorCodes.ts    — code registry completeness and message map
 *   StellarGrantsError — class hierarchy and instanceof checks
 *   parseSorobanError  — all raw input shapes → correct typed error
 *   isContractError    — narrowing helper
 *   getErrorMessage    — lookup helper
 *   Errors.*           — convenience factory functions
 */

import { describe, it, expect } from "vitest";

import {
  ErrorCode,
  ERROR_MESSAGES,
} from "../lib/errors/errorCodes";
import type { ErrorCodeValue } from "../lib/errors/errorCodes";

import {
  StellarGrantsError,
  SorobanContractError,
  StellarGrantsNetworkError,
  Errors,
} from "../lib/errors/StellarGrantsError";

import {
  parseSorobanError,
  isContractError,
  getErrorMessage,
} from "../lib/errors/parseSorobanError";

// ── errorCodes ────────────────────────────────────────────────────────────────

describe("ErrorCode registry", () => {
  it("every ErrorCode value has an entry in ERROR_MESSAGES", () => {
    const codes = Object.values(ErrorCode) as ErrorCodeValue[];
    for (const code of codes) {
      expect(
        ERROR_MESSAGES[code],
        `Missing message for ErrorCode ${code}`
      ).toBeTruthy();
    }
  });

  it("all ERROR_MESSAGES values are non-empty strings", () => {
    for (const [, msg] of Object.entries(ERROR_MESSAGES)) {
      expect(typeof msg).toBe("string");
      expect((msg as string).length).toBeGreaterThan(0);
    }
  });

  it("has the expected key codes", () => {
    expect(ErrorCode.Unauthorized).toBe(1);
    expect(ErrorCode.GrantNotFound).toBe(10);
    expect(ErrorCode.InsufficientFunds).toBe(20);
    expect(ErrorCode.MilestoneNotFound).toBe(30);
    expect(ErrorCode.DelegateAlreadySet).toBe(40);
    expect(ErrorCode.ProfileNotFound).toBe(50);
    expect(ErrorCode.AdminNotSet).toBe(60);
  });
});

// ── StellarGrantsError class hierarchy ───────────────────────────────────────

describe("StellarGrantsError", () => {
  it("is an instance of Error", () => {
    const e = new StellarGrantsError("test");
    expect(e).toBeInstanceOf(Error);
  });

  it("has correct name", () => {
    expect(new StellarGrantsError("x").name).toBe("StellarGrantsError");
  });

  it("preserves the message", () => {
    expect(new StellarGrantsError("hello").message).toBe("hello");
  });
});

describe("SorobanContractError", () => {
  it("extends StellarGrantsError", () => {
    const e = new SorobanContractError(ErrorCode.Unauthorized);
    expect(e).toBeInstanceOf(StellarGrantsError);
    expect(e).toBeInstanceOf(SorobanContractError);
  });

  it("sets message from ERROR_MESSAGES", () => {
    const e = new SorobanContractError(ErrorCode.GrantNotFound);
    expect(e.message).toBe(ERROR_MESSAGES[ErrorCode.GrantNotFound]);
  });

  it("stores the code", () => {
    const e = new SorobanContractError(ErrorCode.InsufficientFunds);
    expect(e.code).toBe(ErrorCode.InsufficientFunds);
  });

  it("stores sorobanDetails", () => {
    const raw = { detail: "raw" };
    const e = new SorobanContractError(ErrorCode.Unauthorized, raw);
    expect(e.sorobanDetails).toBe(raw);
  });

  it("has correct name", () => {
    expect(new SorobanContractError(ErrorCode.GrantNotFound).name).toBe("SorobanContractError");
  });
});

describe("StellarGrantsNetworkError", () => {
  it("extends StellarGrantsError", () => {
    const e = new StellarGrantsNetworkError("timeout");
    expect(e).toBeInstanceOf(StellarGrantsError);
    expect(e).toBeInstanceOf(StellarGrantsNetworkError);
  });

  it("stores statusCode", () => {
    const e = new StellarGrantsNetworkError("bad gateway", { statusCode: 502 });
    expect(e.statusCode).toBe(502);
  });

  it("statusCode is undefined when not provided", () => {
    expect(new StellarGrantsNetworkError("err").statusCode).toBeUndefined();
  });
});

// ── parseSorobanError ─────────────────────────────────────────────────────────

describe("parseSorobanError", () => {
  it("passes through an existing StellarGrantsError unchanged", () => {
    const original = new StellarGrantsError("already typed");
    expect(parseSorobanError(original)).toBe(original);
  });

  it("passes through an existing SorobanContractError unchanged", () => {
    const original = new SorobanContractError(ErrorCode.Unauthorized);
    expect(parseSorobanError(original)).toBe(original);
  });

  // stellar-sdk ≥ 11 shape
  it("maps { type: 'contract', value: N } to SorobanContractError", () => {
    const raw = { type: "contract", value: ErrorCode.GrantNotFound };
    const result = parseSorobanError(raw);
    expect(result).toBeInstanceOf(SorobanContractError);
    expect((result as SorobanContractError).code).toBe(ErrorCode.GrantNotFound);
  });

  it("maps unknown { type: 'contract', value: N } to StellarGrantsError with fallback", () => {
    const raw = { type: "contract", value: 9999 };
    const result = parseSorobanError(raw);
    expect(result).toBeInstanceOf(StellarGrantsError);
    expect(result.message).toContain("9999");
  });

  // Numeric code field
  it("maps { code: N } shape to SorobanContractError", () => {
    const raw = { code: ErrorCode.InsufficientFunds, extra: "data" };
    const result = parseSorobanError(raw);
    expect(result).toBeInstanceOf(SorobanContractError);
    expect((result as SorobanContractError).code).toBe(ErrorCode.InsufficientFunds);
  });

  it("maps unknown { code: N } to generic StellarGrantsError", () => {
    const result = parseSorobanError({ code: 8888 });
    expect(result).toBeInstanceOf(StellarGrantsError);
    expect(result.message).toContain("8888");
  });

  // Horizon HTTP errors
  it("maps { status: 404 } to StellarGrantsNetworkError", () => {
    const raw = { status: 404, message: "Not Found" };
    const result = parseSorobanError(raw);
    expect(result).toBeInstanceOf(StellarGrantsNetworkError);
    expect((result as StellarGrantsNetworkError).statusCode).toBe(404);
    expect(result.message).toBe("Not Found");
  });

  it("provides a fallback message when Horizon status has no message", () => {
    const result = parseSorobanError({ status: 500 });
    expect(result).toBeInstanceOf(StellarGrantsNetworkError);
    expect(result.message).toContain("500");
  });

  // Error message string parsing
  it("maps Error('Error(Contract, #10)') to SorobanContractError", () => {
    const result = parseSorobanError(new Error("Error(Contract, #10)"));
    expect(result).toBeInstanceOf(SorobanContractError);
    expect((result as SorobanContractError).code).toBe(ErrorCode.GrantNotFound);
  });

  it("maps Error('contract error 20') to SorobanContractError", () => {
    const result = parseSorobanError(new Error("contract error 20"));
    expect(result).toBeInstanceOf(SorobanContractError);
    expect((result as SorobanContractError).code).toBe(ErrorCode.InsufficientFunds);
  });

  it("maps plain string with #N pattern to SorobanContractError", () => {
    const result = parseSorobanError("Error(Contract, #1)");
    expect(result).toBeInstanceOf(SorobanContractError);
    expect((result as SorobanContractError).code).toBe(ErrorCode.Unauthorized);
  });

  it("maps an unrecognised Error to generic StellarGrantsError", () => {
    const result = parseSorobanError(new Error("something random happened"));
    expect(result).toBeInstanceOf(StellarGrantsError);
    expect(result.message).toBe("something random happened");
  });

  it("maps a plain string to generic StellarGrantsError", () => {
    const result = parseSorobanError("just a string");
    expect(result).toBeInstanceOf(StellarGrantsError);
    expect(result.message).toBe("just a string");
  });

  it("maps null to generic StellarGrantsError with fallback message", () => {
    const result = parseSorobanError(null);
    expect(result).toBeInstanceOf(StellarGrantsError);
    expect(result.message).toContain("unexpected");
  });

  it("maps undefined to generic StellarGrantsError with fallback message", () => {
    const result = parseSorobanError(undefined);
    expect(result).toBeInstanceOf(StellarGrantsError);
  });

  // Specific named errors
  it("correctly maps Unauthorized (#1)", () => {
    const result = parseSorobanError({ code: ErrorCode.Unauthorized });
    expect((result as SorobanContractError).code).toBe(ErrorCode.Unauthorized);
    expect(result.message).toMatch(/authoris/i);
  });

  it("correctly maps ContractPaused (#2)", () => {
    const result = parseSorobanError({ code: ErrorCode.ContractPaused });
    expect((result as SorobanContractError).code).toBe(ErrorCode.ContractPaused);
    expect(result.message).toMatch(/paused/i);
  });

  it("correctly maps GrantNotFound (#10)", () => {
    const result = parseSorobanError({ code: ErrorCode.GrantNotFound });
    expect((result as SorobanContractError).code).toBe(ErrorCode.GrantNotFound);
    expect(result.message).toMatch(/grant not found/i);
  });

  it("correctly maps InsufficientFunds (#20)", () => {
    const result = parseSorobanError({ code: ErrorCode.InsufficientFunds });
    expect((result as SorobanContractError).code).toBe(ErrorCode.InsufficientFunds);
    expect(result.message).toMatch(/insufficient funds/i);
  });

  it("correctly maps MilestoneAlreadyApproved (#31)", () => {
    const result = parseSorobanError({ code: ErrorCode.MilestoneAlreadyApproved });
    expect((result as SorobanContractError).code).toBe(ErrorCode.MilestoneAlreadyApproved);
    expect(result.message).toMatch(/already been approved/i);
  });

  it("correctly maps QuorumNotReached (#33)", () => {
    const result = parseSorobanError({ code: ErrorCode.QuorumNotReached });
    expect((result as SorobanContractError).code).toBe(ErrorCode.QuorumNotReached);
    expect(result.message).toMatch(/quorum/i);
  });

  it("correctly maps AlreadyVoted (#37)", () => {
    const result = parseSorobanError({ code: ErrorCode.AlreadyVoted });
    expect((result as SorobanContractError).code).toBe(ErrorCode.AlreadyVoted);
    expect(result.message).toMatch(/already voted/i);
  });

  it("correctly maps HardCapExceeded (#16)", () => {
    const result = parseSorobanError({ code: ErrorCode.HardCapExceeded });
    expect((result as SorobanContractError).code).toBe(ErrorCode.HardCapExceeded);
    expect(result.message).toMatch(/cap/i);
  });
});

// ── isContractError ───────────────────────────────────────────────────────────

describe("isContractError", () => {
  it("returns true for a matching SorobanContractError", () => {
    const err = new SorobanContractError(ErrorCode.GrantNotFound);
    expect(isContractError(err, ErrorCode.GrantNotFound)).toBe(true);
  });

  it("returns false for a different code", () => {
    const err = new SorobanContractError(ErrorCode.GrantNotFound);
    expect(isContractError(err, ErrorCode.Unauthorized)).toBe(false);
  });

  it("returns false for a non-contract StellarGrantsError", () => {
    const err = new StellarGrantsError("generic");
    expect(isContractError(err, ErrorCode.GrantNotFound)).toBe(false);
  });
});

// ── getErrorMessage ───────────────────────────────────────────────────────────

describe("getErrorMessage", () => {
  it("returns the registered message for a known code", () => {
    expect(getErrorMessage(ErrorCode.GrantNotFound)).toBe(
      ERROR_MESSAGES[ErrorCode.GrantNotFound]
    );
  });

  it("returns a fallback for an unknown code", () => {
    const msg = getErrorMessage(9999);
    expect(msg).toContain("9999");
  });
});

// ── Errors.* convenience factories ────────────────────────────────────────────

describe("Errors convenience factories", () => {
  it("Errors.grantNotFound() produces correct code", () => {
    const e = Errors.grantNotFound();
    expect(e.code).toBe(ErrorCode.GrantNotFound);
  });

  it("Errors.unauthorized() produces correct code", () => {
    expect(Errors.unauthorized().code).toBe(ErrorCode.Unauthorized);
  });

  it("Errors.insufficientFunds() produces correct code", () => {
    expect(Errors.insufficientFunds().code).toBe(ErrorCode.InsufficientFunds);
  });

  it("Errors.contractPaused() produces correct code", () => {
    expect(Errors.contractPaused().code).toBe(ErrorCode.ContractPaused);
  });

  it("Errors.hardCapExceeded() produces correct code", () => {
    expect(Errors.hardCapExceeded().code).toBe(ErrorCode.HardCapExceeded);
  });

  it("Errors.network() produces StellarGrantsNetworkError with statusCode", () => {
    const e = Errors.network("timeout", 503);
    expect(e).toBeInstanceOf(StellarGrantsNetworkError);
    expect(e.statusCode).toBe(503);
    expect(e.message).toBe("timeout");
  });

  it("factory errors carry the raw details in sorobanDetails", () => {
    const raw = { raw: true };
    expect(Errors.grantNotFound(raw).sorobanDetails).toBe(raw);
  });
});
