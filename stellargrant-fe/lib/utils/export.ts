import type { Grant, Milestone } from "@/types";

export interface FunderRecord {
  address: string;
  amount: bigint;
  token: string;
  timestamp: string;
}

function slugifyTitle(title: string): string {
  return title.toLowerCase().replace(/[^a-z0-9]+/g, "-").slice(0, 30);
}

function triggerDownload(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

function milestoneStatus(m: Milestone): string {
  if (m.paid) return "paid";
  if (m.approved) return "approved";
  if (m.submitted) return "submitted";
  return "pending";
}

export function exportGrantAsJSON(grant: Grant, milestones: Milestone[]): void {
  const data = {
    exportedAt: new Date().toISOString(),
    grant: {
      id: grant.id,
      title: grant.title,
      owner: grant.owner,
      recipient: grant.recipient,
      budget: grant.budget.toString(),
      funded: grant.funded.toString(),
      token: grant.token,
      status: grant.status,
      deadline: new Date(Number(grant.deadline) * 1000).toISOString(),
      createdAt: new Date(Number(grant.created_at) * 1000).toISOString(),
      reviewers: grant.reviewers,
    },
    milestones: milestones.map((m) => ({
      index: m.idx,
      title: m.title,
      description: m.description,
      reward: m.amount?.toString() ?? "0",
      token: m.token,
      status: milestoneStatus(m),
      proofHash: m.proof_hash,
      submittedAt: m.submitted_at
        ? new Date(Number(m.submitted_at) * 1000).toISOString()
        : null,
      paidAt: m.paid_at ? new Date(Number(m.paid_at) * 1000).toISOString() : null,
    })),
  };

  const blob = new Blob([JSON.stringify(data, null, 2)], {
    type: "application/json",
  });
  const slug = slugifyTitle(grant.title);
  const date = new Date().toISOString().slice(0, 10);
  triggerDownload(blob, `stellargrant-${grant.id}-${slug}-${date}.json`);
}

export function exportGrantAsCSV(grant: Grant, milestones: Milestone[]): void {
  const header = [
    "Index",
    "Title",
    "Reward (stroops)",
    "Token",
    "Status",
    "Proof Hash",
    "Submitted At",
    "Paid At",
  ].join(",");

  const rows = milestones.map((m) =>
    [
      m.idx,
      `"${m.title.replace(/"/g, '""')}"`,
      m.amount?.toString() ?? "0",
      m.token ?? "native",
      milestoneStatus(m),
      m.proof_hash ?? "",
      m.submitted_at
        ? new Date(Number(m.submitted_at) * 1000).toISOString()
        : "",
      m.paid_at ? new Date(Number(m.paid_at) * 1000).toISOString() : "",
    ].join(",")
  );

  const csv = [header, ...rows].join("\n");
  const blob = new Blob([csv], { type: "text/csv" });
  triggerDownload(blob, `stellargrant-${grant.id}-milestones.csv`);
}

export function exportFundersAsCSV(funders: FunderRecord[]): void {
  const header = ["Address", "Amount (stroops)", "Token", "Timestamp"].join(",");

  const rows = funders.map((f) =>
    [
      f.address,
      f.amount.toString(),
      f.token,
      f.timestamp,
    ].join(",")
  );

  const csv = [header, ...rows].join("\n");
  const blob = new Blob([csv], { type: "text/csv" });
  // grant id is not available here so use a generic name
  triggerDownload(blob, `stellargrant-funders.csv`);
}
