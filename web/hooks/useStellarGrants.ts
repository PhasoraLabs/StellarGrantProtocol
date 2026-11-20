"use client";

/**
 * useStellarGrants Hook
 *
 * Access the StellarGrants SDK context: contract client, logger, and
 * batch builder — all pre-configured from the nearest StellarGrantsProvider.
 */

import { useContext } from "react";
import { StellarGrantsContext } from "@/components/StellarGrantsProvider";

export function useStellarGrants() {
  const ctx = useContext(StellarGrantsContext);
  if (!ctx) {
    throw new Error("useStellarGrants must be used inside <StellarGrantsProvider>");
  }
  return ctx;
}
