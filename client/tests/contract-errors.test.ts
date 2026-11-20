/**
 * @file contract-errors.test.ts
 *
 * Comprehensive unit tests for the contract error mapping system (Issue #464).
 *
 * Coverage:
 *  - ContractErrorCode enum — all 21 variants with correct u32 values
 *  - ErrorMessages — every code has a non-empty message
 *  - ContractError class — message, code string, contractCode, details, instanceof
 *  - parseSorobanError — contract error path, revert path, rpc path, unknown path
 *  - resolving unknown contract codes gracefully
 *  - Soroban message format variations
 */

import { ContractError, SorobanRevertError, StellarGrantsError } from "../src/errors/StellarGrantsError";
import { ContractErrorCode, ErrorMessages } from "../src/errors/errorCodes";
import { parseSorobanError } from "../src/errors/parseSorobanError";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Simulates the error message string emitted by stellar-sdk for a contract error. */
function sorobanContractError(code: number): Error {
  return new Error(`HostError: Error(Contract, #${code})`);
}

// ---------------------------------------------------------------------------
// ContractErrorCode — enum shape
// ---------------------------------------------------------------------------

describe("ContractErrorCode enum", () => {
  const expectedVariants: [string, number][] = [
    ["GrantNotFound",             1],
    ["Unauthorized",              2],
    ["MilestoneAlreadyApproved",  3],
    ["QuorumNotReached",          4],
    ["DeadlinePassed",            5],
    ["InvalidInput",              6],
    ["MilestoneNotSubmitted",     7],
    ["AlreadyVoted",              8],
    ["MilestoneNotFound",         9],
    ["InvalidState",              10],
    ["NoRefundableAmount",        11],
    ["GrantAlreadyReleased",      12],
    ["NotMultisigSigner",         13],
    ["AlreadySignedRelease",      14],
    ["NotAllMilestonesApproved",  15],
    ["InsufficientStake",         16],
    ["StakeNotFound",             17],
    ["AlreadyRegistered",         18],
    ["BatchEmpty",                19],
    ["BatchTooLarge",             20],
    ["MilestoneAlreadySubmitted", 21],
  ];

  it("has exactly 21 variants", () => {
    // Numeric enums produce both name→value and value→name entries;
    // divide by 2 to get the real count.
    const variantCount = Object.keys(ContractErrorCode).filter(k => isNaN(Number(k))).length;
    expect(variantCount).toBe(21);
  });

  it.each(expectedVariants)(
    "ContractErrorCode.%s === %i (matches types.rs #[repr(u32)])",
    (name, expectedValue) => {
      expect(ContractErrorCode[name as keyof typeof ContractErrorCode]).toBe(expectedValue);
    },
  );
});

// ---------------------------------------------------------------------------
// ErrorMessages — completeness
// ---------------------------------------------------------------------------

describe("ErrorMessages", () => {
  it("provides a non-empty message for every ContractErrorCode", () => {
    const numericVariants = Object.values(ContractErrorCode).filter(
      (v): v is ContractErrorCode => typeof v === "number",
    );

    for (const code of numericVariants) {
      const msg = ErrorMessages[code];
      expect(typeof msg).toBe("string");
      expect(msg.length).toBeGreaterThan(0);
    }
  });

  it("has no extra keys beyond the 21 defined codes", () => {
    const messageKeys = Object.keys(ErrorMessages).map(Number);
    const enumValues = Object.values(ContractErrorCode).filter(
      (v): v is number => typeof v === "number",
    );
    expect(messageKeys.sort((a, b) => a - b)).toEqual(enumValues.sort((a, b) => a - b));
  });
});

// ---------------------------------------------------------------------------
// ContractError class
// ---------------------------------------------------------------------------

describe("ContractError", () => {
  it("is an instance of Error, StellarGrantsError, and ContractError", () => {
    const err = new ContractError(ContractErrorCode.GrantNotFound);
    expect(err).toBeInstanceOf(Error);
    expect(err).toBeInstanceOf(StellarGrantsError);
    expect(err).toBeInstanceOf(ContractError);
  });

  it("name is 'ContractError'", () => {
    expect(new ContractError(ContractErrorCode.Unauthorized).name).toBe("ContractError");
  });

  it("message is taken from ErrorMessages", () => {
    const err = new ContractError(ContractErrorCode.Unauthorized);
    expect(err.message).toBe(ErrorMessages[ContractErrorCode.Unauthorized]);
  });

  it("code string encodes the numeric discriminant", () => {
    const err = new ContractError(ContractErrorCode.AlreadyVoted); // = 8
    expect(err.code).toBe("CONTRACT_ERROR_8");
  });

  it("contractCode holds the typed enum value", () => {
    const err = new ContractError(ContractErrorCode.MilestoneNotFound);
    expect(err.contractCode).toBe(ContractErrorCode.MilestoneNotFound);
    expect(err.contractCode).toBe(9);
  });

  it("details stores the sorobanDetails payload", () => {
    const payload = { raw: "HostError: Error(Contract, #1)" };
    const err = new ContractError(ContractErrorCode.GrantNotFound, payload);
    expect(err.details).toEqual(payload);
  });

  it("details is undefined when not supplied", () => {
    const err = new ContractError(ContractErrorCode.BatchEmpty);
    expect(err.details).toBeUndefined();
  });

  // Spot-check a representative set of common errors
  const spotChecks: [ContractErrorCode, string][] = [
    [ContractErrorCode.GrantNotFound,            "not found"],
    [ContractErrorCode.Unauthorized,             "not authorized"],
    [ContractErrorCode.InsufficientStake,        "nsufficien"],    // "Insufficient"
    [ContractErrorCode.GrantAlreadyReleased,     "already been released"],
    [ContractErrorCode.MilestoneAlreadySubmitted,"already been submitted"],
  ];

  it.each(spotChecks)(
    "ContractError(%i) message contains the expected keyword",
    (code, keyword) => {
      const err = new ContractError(code);
      expect(err.message.toLowerCase()).toContain(keyword.toLowerCase());
    },
  );
});

// ---------------------------------------------------------------------------
// parseSorobanError — contract error path
// ---------------------------------------------------------------------------

describe("parseSorobanError — typed contract errors", () => {
  it("returns ContractError for Error(Contract, #1) — GrantNotFound", () => {
    const result = parseSorobanError(sorobanContractError(1));
    expect(result).toBeInstanceOf(ContractError);
    const ce = result as ContractError;
    expect(ce.contractCode).toBe(ContractErrorCode.GrantNotFound);
    expect(ce.message).toBe(ErrorMessages[ContractErrorCode.GrantNotFound]);
  });

  it("returns ContractError for Error(Contract, #2) — Unauthorized", () => {
    const result = parseSorobanError(sorobanContractError(2)) as ContractError;
    expect(result).toBeInstanceOf(ContractError);
    expect(result.contractCode).toBe(ContractErrorCode.Unauthorized);
  });

  it("returns ContractError for Error(Contract, #8) — AlreadyVoted", () => {
    const result = parseSorobanError(sorobanContractError(8)) as ContractError;
    expect(result).toBeInstanceOf(ContractError);
    expect(result.contractCode).toBe(ContractErrorCode.AlreadyVoted);
  });

  it("returns ContractError for Error(Contract, #16) — InsufficientStake", () => {
    const result = parseSorobanError(sorobanContractError(16)) as ContractError;
    expect(result).toBeInstanceOf(ContractError);
    expect(result.contractCode).toBe(ContractErrorCode.InsufficientStake);
  });

  it("returns ContractError for Error(Contract, #21) — MilestoneAlreadySubmitted", () => {
    const result = parseSorobanError(sorobanContractError(21)) as ContractError;
    expect(result).toBeInstanceOf(ContractError);
    expect(result.contractCode).toBe(ContractErrorCode.MilestoneAlreadySubmitted);
  });

  it("attaches raw message to details for debugging", () => {
    const result = parseSorobanError(sorobanContractError(1)) as ContractError;
    const details = result.details as { raw: string; originalError: Error };
    expect(details.raw).toContain("Error(Contract, #1)");
    expect(details.originalError).toBeInstanceOf(Error);
  });

  it("maps all 21 known codes correctly (exhaustive)", () => {
    const expected: [number, ContractErrorCode][] = [
      [1,  ContractErrorCode.GrantNotFound],
      [2,  ContractErrorCode.Unauthorized],
      [3,  ContractErrorCode.MilestoneAlreadyApproved],
      [4,  ContractErrorCode.QuorumNotReached],
      [5,  ContractErrorCode.DeadlinePassed],
      [6,  ContractErrorCode.InvalidInput],
      [7,  ContractErrorCode.MilestoneNotSubmitted],
      [8,  ContractErrorCode.AlreadyVoted],
      [9,  ContractErrorCode.MilestoneNotFound],
      [10, ContractErrorCode.InvalidState],
      [11, ContractErrorCode.NoRefundableAmount],
      [12, ContractErrorCode.GrantAlreadyReleased],
      [13, ContractErrorCode.NotMultisigSigner],
      [14, ContractErrorCode.AlreadySignedRelease],
      [15, ContractErrorCode.NotAllMilestonesApproved],
      [16, ContractErrorCode.InsufficientStake],
      [17, ContractErrorCode.StakeNotFound],
      [18, ContractErrorCode.AlreadyRegistered],
      [19, ContractErrorCode.BatchEmpty],
      [20, ContractErrorCode.BatchTooLarge],
      [21, ContractErrorCode.MilestoneAlreadySubmitted],
    ];

    for (const [numCode, enumCode] of expected) {
      const result = parseSorobanError(sorobanContractError(numCode)) as ContractError;
      expect(result).toBeInstanceOf(ContractError);
      expect(result.contractCode).toBe(enumCode);
    }
  });

  it("handles various Soroban message prefixes containing Error(Contract, #N)", () => {
    const variants = [
      new Error("Error(Contract, #2)"),
      new Error("HostError: Error(Contract, #2)"),
      new Error("TransactionSubmitErrors: [Error(Contract, #2)]"),
      new Error("simulation failed: Error(Contract, #2) at some ledger"),
      new Error("Error( Contract , # 2 )"), // extra whitespace (tolerant parsing)
    ];

    for (const err of variants) {
      const result = parseSorobanError(err) as ContractError;
      expect(result).toBeInstanceOf(ContractError);
      expect(result.contractCode).toBe(ContractErrorCode.Unauthorized);
    }
  });

  it("returns a StellarGrantsError with CONTRACT_ERROR_N code for an unknown contract code", () => {
    // Code 999 is not in the enum
    const result = parseSorobanError(new Error("HostError: Error(Contract, #999)")) as StellarGrantsError;
    expect(result).toBeInstanceOf(StellarGrantsError);
    expect(result).not.toBeInstanceOf(ContractError);
    expect(result.code).toBe("CONTRACT_ERROR_999");
    expect(result.message).toContain("999");
  });
});

// ---------------------------------------------------------------------------
// parseSorobanError — revert / txFailed path (regression: must still work)
// ---------------------------------------------------------------------------

describe("parseSorobanError — revert / txFailed (regression)", () => {
  it("returns SorobanRevertError for a revert message without a contract code", () => {
    const result = parseSorobanError(new Error("Contract revert: not active"));
    expect(result).toBeInstanceOf(SorobanRevertError);
    expect(result.message).toContain("not active");
  });

  it("returns SorobanRevertError for txFailed without a contract code", () => {
    const result = parseSorobanError(new Error("txFailed: bad state"));
    expect(result).toBeInstanceOf(SorobanRevertError);
  });

  it("attaches raw message and originalError to SorobanRevertError.details", () => {
    const original = new Error("Contract revert: something");
    const result = parseSorobanError(original) as StellarGrantsError;
    const details = result.details as { raw: string; originalError: Error };
    expect(details.raw).toBe(original.message);
    expect(details.originalError).toBe(original);
  });
});

// ---------------------------------------------------------------------------
// parseSorobanError — RPC / unknown paths (regression)
// ---------------------------------------------------------------------------

describe("parseSorobanError — RPC and unknown paths (regression)", () => {
  it("returns StellarGrantsError with RPC_ERROR for a plain network Error", () => {
    const result = parseSorobanError(new Error("network timeout")) as StellarGrantsError;
    expect(result).toBeInstanceOf(StellarGrantsError);
    expect(result).not.toBeInstanceOf(ContractError);
    expect(result).not.toBeInstanceOf(SorobanRevertError);
    expect(result.code).toBe("RPC_ERROR");
    expect(result.message).toBe("network timeout");
  });

  it("returns StellarGrantsError with UNKNOWN_RPC_ERROR for a string throw", () => {
    const result = parseSorobanError("some string") as StellarGrantsError;
    expect(result.code).toBe("UNKNOWN_RPC_ERROR");
  });

  it("returns StellarGrantsError with UNKNOWN_RPC_ERROR for null", () => {
    const result = parseSorobanError(null) as StellarGrantsError;
    expect(result.code).toBe("UNKNOWN_RPC_ERROR");
  });

  it("returns StellarGrantsError with UNKNOWN_RPC_ERROR for a plain object", () => {
    const result = parseSorobanError({ status: 500 }) as StellarGrantsError;
    expect(result.code).toBe("UNKNOWN_RPC_ERROR");
  });
});
