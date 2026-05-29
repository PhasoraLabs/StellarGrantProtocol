/**
 * Grant Export Utilities
 *
 * Provides functions to export grant data as JSON or CSV files.
 * Works without a wallet connection (read-only feature).
 *
 * @see https://github.com/StellarGrant/StellarGrant-fe/issues/388
 */

import type { Grant, Milestone } from "@/types";

// ── Types ────────────────────────────────────────────────────────────────────

/** A record representing a funder/contributor to a grant. */
export interface FunderRecord {
  address: string;
  amount: bigint;
  token: string;
  timestamp: bigint | null;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Slug a grant title for use in file names.
 * Lowercases, replaces non-alphanumeric chars with hyphens, truncates to 30 chars.
 */
function slugTitle(title: string): string {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 30);
}

/**
 * Trigger a browser download for the given content.
 */
function downloadFile(filename: string, content: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

/**
 * Convert a bigint to string safely (handles BigInt serialization).
 */
function bigintToString(value: bigint | null | undefined): string {
  if (value === null || value === undefined) return "0";
  return value.toString();
}

/**
 * Format a bigint timestamp (seconds since epoch) to ISO string.
 */
function timestampToISO(ts: bigint | null): string | null {
  if (ts === null || ts === undefined) return null;
  try {
    return new Date(Number(ts) * 1000).toISOString();
  } catch {
    return null;
  }
}

/**
 * Derive the status string from milestone boolean flags.
 */
function milestoneStatus(m: Milestone): string {
  if (m.paid) return "paid";
  if (m.approved) return "approved";
  if (m.submitted) return "submitted";
  return "pending";
}

// ── JSON Export ──────────────────────────────────────────────────────────────

/**
 * Export a grant and its milestones as a JSON file.
 * BigInt values are serialized as strings to avoid JSON.stringify errors.
 */
export function exportGrantAsJSON(grant: Grant, milestones: Milestone[]): void {
  const data = {
    exportedAt: new Date().toISOString(),
    grant: {
      id: grant.id,
      title: grant.title,
      owner: grant.owner,
      recipient: grant.recipient,
      budget: bigintToString(grant.budget),
      funded: bigintToString(grant.funded),
      token: grant.token ?? "native",
      status: grant.status,
      deadline: new Date(Number(grant.deadline) * 1000).toISOString(),
      createdAt: new Date(Number(grant.created_at) * 1000).toISOString(),
      reviewers: grant.reviewers,
    },
    milestones: milestones.map((m) => ({
      index: m.idx,
      title: m.title,
      description: m.description,
      reward: bigintToString(m.amount),
      token: m.token ?? "native",
      status: milestoneStatus(m),
      proofHash: m.proof_hash,
      submittedAt: timestampToISO(m.submitted_at),
      paidAt: timestampToISO(m.paid_at),
    })),
  };

  const json = JSON.stringify(data, null, 2);
  const filename = `stellargrant-${grant.id}-${slugTitle(grant.title)}-${Date.now()}.json`;
  downloadFile(filename, json, "application/json");
}

// ── Milestones CSV Export ────────────────────────────────────────────────────

/**
 * Escape a CSV field value (wraps in quotes if it contains commas, quotes, or newlines).
 */
function escapeCSV(value: string): string {
  if (value.includes(",") || value.includes('"') || value.includes("\n")) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

/**
 * Export milestones as a CSV file.
 */
export function exportGrantAsCSV(grant: Grant, milestones: Milestone[]): void {
  const headers = [
    "Index",
    "Title",
    "Reward (stroops)",
    "Token",
    "Status",
    "Proof Hash",
    "Submitted At",
    "Paid At",
  ];

  const rows = milestones.map((m) => [
    m.idx.toString(),
    escapeCSV(m.title),
    bigintToString(m.amount),
    m.token ?? "native",
    milestoneStatus(m),
    m.proof_hash ?? "",
    timestampToISO(m.submitted_at) ?? "",
    timestampToISO(m.paid_at) ?? "",
  ]);

  const csv = [headers.join(","), ...rows.map((r) => r.join(","))].join("\n");
  const filename = `stellargrant-${grant.id}-milestones.csv`;
  downloadFile(filename, csv, "text/csv");
}

// ── Funders CSV Export ───────────────────────────────────────────────────────

/**
 * Export funders as a CSV file.
 */
export function exportFundersAsCSV(funders: FunderRecord[]): void {
  const headers = ["Address", "Amount (stroops)", "Token", "Timestamp"];

  const rows = funders.map((f) => [
    f.address,
    bigintToString(f.amount),
    f.token ?? "native",
    timestampToISO(f.timestamp) ?? "",
  ]);

  const csv = [headers.join(","), ...rows.map((r) => r.join(","))].join("\n");
  const filename = `stellargrant-funders-${Date.now()}.csv`;
  downloadFile(filename, csv, "text/csv");
}
