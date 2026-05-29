import type { Metadata } from "next";
import { fetchGrantById } from "@/lib/grants/api";
import GrantDetailClient from "./GrantDetailClient";

interface GrantDetailPageProps {
  params: Promise<{ id: string }>;
}

export async function generateMetadata({ params }: GrantDetailPageProps): Promise<Metadata> {
  const { id } = await params;
  const detail = await fetchGrantById(id);
  const grant = detail?.grant;
  const description = grant?.description?.slice(0, 160) ?? "";

  return {
    title: `${grant?.title ?? "Grant"} — StellarGrant Protocol`,
    description,
    openGraph: {
      title: grant?.title,
      description,
    },
  };
}

export default function GrantDetailPage({ params }: GrantDetailPageProps) {
  return <GrantDetailClient params={params} />;
}

export const dynamic = "force-dynamic";
