import type { Metadata } from "next";
import Link from "next/link";
import { WalletAddress } from "@/components/wallet/WalletAddress";
import { API_URL } from "@/lib/constants";
import { addressToColor } from "@/lib/search/map";

interface ContributorPageProps {
  params: Promise<{ address: string }>;
}

export async function generateMetadata({ params }: ContributorPageProps): Promise<Metadata> {
  const { address } = await params;
  return {
    title: `${address.slice(0, 8)}… — Contributor — StellarGrant Protocol`,
  };
}

export default async function ContributorPage({ params }: ContributorPageProps) {
  const { address } = await params;

  let reputation = 0;
  let grantsCompleted = 0;
  let bio: string | null = null;

  try {
    const res = await fetch(`${API_URL}/profiles/${encodeURIComponent(address)}`, {
      next: { revalidate: 60 },
    });
    if (res.ok) {
      const json = (await res.json()) as {
        data?: { bio?: string | null; grants?: unknown[] };
      };
      bio = json.data?.bio ?? null;
      grantsCompleted = json.data?.grants?.length ?? 0;
    }
  } catch {
    // Profile may not exist yet — still render the address page.
  }

  try {
    const leaderboardRes = await fetch(`${API_URL}/leaderboard?limit=100`, {
      next: { revalidate: 60 },
    });
    if (leaderboardRes.ok) {
      const json = (await leaderboardRes.json()) as {
        data?: Array<{ address: string; reputation: number; totalGrantsCompleted?: number }>;
      };
      const entry = (json.data ?? []).find((row) => row.address === address);
      if (entry) {
        reputation = entry.reputation;
        grantsCompleted = entry.totalGrantsCompleted ?? grantsCompleted;
      }
    }
  } catch {
    // Leaderboard optional.
  }

  return (
    <div className="container mx-auto max-w-3xl px-4 py-8">
      <Link
        href="/search"
        className="font-mono text-xs text-accent-secondary hover:underline"
      >
        ← Back to Search
      </Link>

      <div className="mt-6 flex items-start gap-4 border border-border-color bg-surface p-6 ring-1 ring-border-color">
        <div
          className="flex h-12 w-12 shrink-0 items-center justify-center font-mono text-sm font-bold text-white"
          style={{ backgroundColor: addressToColor(address) }}
          aria-hidden="true"
        >
          {address.slice(0, 2)}
        </div>
        <div className="min-w-0 space-y-3">
          <WalletAddress address={address} />
          <div className="flex flex-wrap gap-4 font-mono text-xs text-text-muted">
            <span>
              Score: <span className="text-text-primary">{reputation}</span>
            </span>
            <span>
              {grantsCompleted} grant{grantsCompleted === 1 ? "" : "s"} completed
            </span>
          </div>
          {bio && <p className="font-mono text-sm text-text-primary">{bio}</p>}
        </div>
      </div>
    </div>
  );
}

export const dynamic = "force-dynamic";
