import { Metadata } from "next";
import DisputePanelClient from "@/components/dispute/DisputePanelClient";

export const metadata: Metadata = {
  title: "Council Dispute Panel — StellarGrant Protocol",
  description: "Council dispute resolution panel for StellarGrant Protocol milestones.",
};

export default function DisputePage() {
  return (
    <div className="container mx-auto px-4 py-8 max-w-5xl">
      <DisputePanelClient />
    </div>
  );
}
