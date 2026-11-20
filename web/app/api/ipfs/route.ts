import { NextResponse } from "next/server";

const PINATA_URL = "https://api.pinata.cloud/pinning/pinFileToIPFS";
const MAX_FILE_SIZE = 10 * 1024 * 1024;

export const runtime = "nodejs";

export async function POST(request: Request) {
  const pinataJwt = process.env.PINATA_JWT;
  if (!pinataJwt) {
    return NextResponse.json(
      { error: "IPFS uploads are not configured" },
      { status: 503 },
    );
  }

  let formData: FormData;
  try {
    formData = await request.formData();
  } catch {
    return NextResponse.json({ error: "Invalid multipart upload" }, { status: 400 });
  }

  const file = formData.get("file");
  if (!(file instanceof File)) {
    return NextResponse.json({ error: "A file is required" }, { status: 400 });
  }
  if (file.size > MAX_FILE_SIZE) {
    return NextResponse.json({ error: "File exceeds 10 MB limit" }, { status: 413 });
  }

  try {
    const response = await fetch(PINATA_URL, {
      method: "POST",
      headers: { Authorization: `Bearer ${pinataJwt}` },
      body: formData,
      cache: "no-store",
    });
    const body = (await response.json()) as { IpfsHash?: string; error?: string };

    if (!response.ok || !body.IpfsHash) {
      return NextResponse.json(
        { error: body.error ?? "Pinata upload failed" },
        { status: response.ok ? 502 : response.status },
      );
    }

    return NextResponse.json({ IpfsHash: body.IpfsHash });
  } catch {
    return NextResponse.json(
      { error: "Failed to reach the IPFS provider" },
      { status: 502 },
    );
  }
}
