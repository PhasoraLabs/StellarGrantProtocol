import { proxyGet } from "@/lib/apiProxy";

export async function GET(
  request: Request,
  context: { params: Promise<{ id: string; idx: string }> },
) {
  const { id, idx } = await context.params;
  return proxyGet(
    request,
    `/grants/${encodeURIComponent(id)}/milestones/${encodeURIComponent(idx)}`,
    { revalidate: 30 },
  );
}
