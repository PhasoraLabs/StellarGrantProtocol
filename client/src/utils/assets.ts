import { nativeToScVal, xdr } from "@stellar/stellar-sdk";

export const NATIVE_XLM_ADDRESS = "CDLZFC3SYJYDZT7S64ZDSBKDONEXVYSE5EMGMF7NICMXA3GOF3LBPWB2";

export function isNativeXLM(tokenAddress: string): boolean {
  return tokenAddress === NATIVE_XLM_ADDRESS;
}

export function toAssetScVal(tokenAddress: string): xdr.ScVal {
  if (isNativeXLM(tokenAddress)) {
    return nativeToScVal(NATIVE_XLM_ADDRESS, { type: "address" });
  }
  return nativeToScVal(tokenAddress, { type: "address" });
}
