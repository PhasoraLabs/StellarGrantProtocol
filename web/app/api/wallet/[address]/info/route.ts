import { proxyGet } from "@/lib/apiProxy";

export async function GET(
  request: Request,
  context: { params: Promise<{ address: string }> },
) {
  const { address } = await context.params;
  return proxyGet(
    request,
    `/wallet/${encodeURIComponent(address)}/info`,
    { revalidate: 10, forwardSearch: false },
  );
}
