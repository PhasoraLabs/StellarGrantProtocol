import { StellarGrantsError, SorobanRevertError } from "../src/errors/StellarGrantsError";
import { parseSorobanError } from "../src/errors/parseSorobanError";

// ---------------------------------------------------------------------------
// StellarGrantsError constructor — Req 6.1, 6.2
// ---------------------------------------------------------------------------
describe("StellarGrantsError", () => {
    it("has correct name, message, and default code", () => {
        const err = new StellarGrantsError("something went wrong");
        expect(err.name).toBe("StellarGrantsError");
        expect(err.message).toBe("something went wrong");
        expect(err.code).toBe("STELLAR_GRANTS_ERROR");
        expect(err.details).toBeUndefined();
    });

    it("accepts a custom code and details", () => {
        const details = { raw: "raw error" };
        const err = new StellarGrantsError("msg", "MY_CODE", details);
        expect(err.code).toBe("MY_CODE");
        expect(err.details).toEqual(details);
    });

    it("is an instance of Error", () => {
        expect(new StellarGrantsError("x")).toBeInstanceOf(Error);
    });
});

// ---------------------------------------------------------------------------
// SorobanRevertError constructor — Req 6.3, 6.4
// ---------------------------------------------------------------------------
describe("SorobanRevertError", () => {
    it("has correct name and code", () => {
        const err = new SorobanRevertError("reverted");
        expect(err.name).toBe("SorobanRevertError");
        expect(err.code).toBe("SOROBAN_REVERT");
        expect(err.message).toBe("reverted");
    });

    it("is instanceof StellarGrantsError and SorobanRevertError (Property 9)", () => {
        const err = new SorobanRevertError("x");
        expect(err).toBeInstanceOf(SorobanRevertError);
        expect(err).toBeInstanceOf(StellarGrantsError);
        expect(err).toBeInstanceOf(Error);
    });

    it("accepts details", () => {
        const err = new SorobanRevertError("msg", { raw: "raw" });
        expect(err.details).toEqual({ raw: "raw" });
    });
});

// ---------------------------------------------------------------------------
// parseSorobanError — Req 5.1–5.4
// ---------------------------------------------------------------------------
describe("parseSorobanError", () => {
    it('returns SorobanRevertError for Error containing "revert"', () => {
        const result = parseSorobanError(new Error("Contract revert: not active"));
        expect(result).toBeInstanceOf(SorobanRevertError);
        expect(result.message).toContain("not active");
    });

    it('returns SorobanRevertError for Error containing "txFailed" (mixed case)', () => {
        const result = parseSorobanError(new Error("txFailed: bad state"));
        expect(result).toBeInstanceOf(SorobanRevertError);
    });

    it('returns SorobanRevertError for Error containing "REVERT" (uppercase)', () => {
        const result = parseSorobanError(new Error("REVERT: unauthorized"));
        expect(result).toBeInstanceOf(SorobanRevertError);
    });

    it('returns SorobanRevertError for Error containing "txfailed" (lowercase)', () => {
        const result = parseSorobanError(new Error("txfailed: insufficient funds"));
        expect(result).toBeInstanceOf(SorobanRevertError);
    });

    it('returns StellarGrantsError with code RPC_ERROR for plain Error', () => {
        const result = parseSorobanError(new Error("network timeout")) as StellarGrantsError;
        expect(result).toBeInstanceOf(StellarGrantsError);
        expect(result).not.toBeInstanceOf(SorobanRevertError);
        expect(result.code).toBe("RPC_ERROR");
        expect(result.message).toBe("network timeout");
    });

    it('returns StellarGrantsError with code UNKNOWN_RPC_ERROR for non-Error string', () => {
        const result = parseSorobanError("some string error") as StellarGrantsError;
        expect(result).toBeInstanceOf(StellarGrantsError);
        expect(result.code).toBe("UNKNOWN_RPC_ERROR");
    });

    it('returns StellarGrantsError with code UNKNOWN_RPC_ERROR for non-Error object', () => {
        const result = parseSorobanError({ code: 500 }) as StellarGrantsError;
        expect(result).toBeInstanceOf(StellarGrantsError);
        expect(result.code).toBe("UNKNOWN_RPC_ERROR");
    });

    it('returns StellarGrantsError with code UNKNOWN_RPC_ERROR for null', () => {
        const result = parseSorobanError(null) as StellarGrantsError;
        expect(result.code).toBe("UNKNOWN_RPC_ERROR");
    });

    // Property 2: classification rule holds for all branches
    it("Property 2 — classification rule: revert/txfailed → SorobanRevertError, other Error → RPC_ERROR, non-Error → UNKNOWN_RPC_ERROR", () => {
        const revertCases = [
            "revert",
            "txfailed",
            "REVERT",
            "TXFAILED",
            "HostError: txFailed: something",
            "Contract reverted: bad",
        ];
        for (const msg of revertCases) {
            const r = parseSorobanError(new Error(msg));
            expect(r).toBeInstanceOf(SorobanRevertError);
        }

        const rpcCases = ["network error", "timeout", "connection refused", "404 not found"];
        for (const msg of rpcCases) {
            const r = parseSorobanError(new Error(msg)) as StellarGrantsError;
            expect(r).toBeInstanceOf(StellarGrantsError);
            expect(r).not.toBeInstanceOf(SorobanRevertError);
            expect(r.code).toBe("RPC_ERROR");
        }

        const unknownCases = [null, undefined, 42, "string", { obj: true }];
        for (const val of unknownCases) {
            const r = parseSorobanError(val) as StellarGrantsError;
            expect(r.code).toBe("UNKNOWN_RPC_ERROR");
        }
    });
});
