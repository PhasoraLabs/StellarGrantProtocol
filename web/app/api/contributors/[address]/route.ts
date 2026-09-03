import { proxyGet } from "@/lib/apiProxy";

export async function GET(
  request: Request,
  context: { params: Promise<{ address: string }> },
) {
  const { address } = await context.params;
  return proxyGet(request, `/contributors/${encodeURIComponent(address)}`, {
    revalidate: 120,
  });
}
