import type { Metadata } from "next";
import { use } from "react";
import { GrantHistoryClient } from "./GrantHistoryClient";

export const metadata: Metadata = {
  title: "Transaction History — StellarGrant Protocol",
};

interface GrantHistoryPageProps {
  params: Promise<{ id: string }>;
}

export default function GrantHistoryPage({ params }: GrantHistoryPageProps) {
  const { id } = use(params);
  return <GrantHistoryClient grantId={id} />;
}

export const dynamic = "force-dynamic";
