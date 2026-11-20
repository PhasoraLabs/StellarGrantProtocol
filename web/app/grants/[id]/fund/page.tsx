/**
 * Fund Grant Page
 *
 * Dedicated funding flow page. Lets any wallet holder deposit XLM or USDC
 * into a grant's escrow. Deep-linkable at /grants/[id]/fund.
 */

import { Metadata } from "next";
import { FundGrantClient } from "./FundGrantClient";

export const metadata: Metadata = {
  title: "Fund Grant — StellarGrant Protocol",
};

export default async function FundGrantPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;

  return <FundGrantClient grantId={id} />;
}
