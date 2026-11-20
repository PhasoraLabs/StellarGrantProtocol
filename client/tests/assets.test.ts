import { isNativeXLM, NATIVE_XLM_ADDRESS } from "../src/utils/assets";

describe("assets utils", () => {
  it("isNativeXLM returns true for the native XLM address", () => {
    expect(isNativeXLM(NATIVE_XLM_ADDRESS)).toBe(true);
  });

  it("isNativeXLM returns false for other addresses", () => {
    expect(isNativeXLM("CDOTHERTOKEN")).toBe(false);
    expect(isNativeXLM("GBTEST")).toBe(false);
  });
});
