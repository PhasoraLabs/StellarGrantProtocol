import { Readable } from "stream";
import FormData from "form-data";
import { env } from "../config/env";
import { logger } from "../config/logger";

export interface IpfsUploadResult {
  cid: string;
  gatewayUrl: string;
  size: number;
}

export class IpfsService {
  private readonly pinataApiUrl = "https://api.pinata.cloud/pinning/pinFileToIPFS";
  private readonly pinataJwt: string;
  private readonly gatewayBase: string;

  constructor() {
    this.pinataJwt = env.pinataJwt;
    this.gatewayBase = env.ipfsGateway;
  }

  async uploadFile(
    fileBuffer: Buffer,
    fileName: string,
    mimeType: string,
  ): Promise<IpfsUploadResult> {
    const form = new FormData();
    form.append("file", Readable.from(fileBuffer), {
      filename: fileName,
      contentType: mimeType,
      knownLength: fileBuffer.length,
    });
    form.append(
      "pinataMetadata",
      JSON.stringify({ name: fileName }),
      { contentType: "application/json" },
    );

    const headers: Record<string, string> = {
      ...(form.getHeaders() as Record<string, string>),
      Authorization: `Bearer ${this.pinataJwt}`,
    };

    const response = await fetch(this.pinataApiUrl, {
      method: "POST",
      headers,
      body: form as unknown as BodyInit,
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`IPFS upload failed (${response.status}): ${text}`);
    }

    const json = (await response.json()) as { IpfsHash: string; PinSize: number };
    const cid = json.IpfsHash;

    return {
      cid,
      gatewayUrl: `${this.gatewayBase}/ipfs/${cid}`,
      size: json.PinSize,
    };
  }

  gatewayUrl(cid: string): string {
    return `${this.gatewayBase}/ipfs/${cid}`;
  }

  /**
   * Issues a short-lived, single-use Pinata JWT so the frontend can upload
   * files directly to Pinata without routing bytes through this API server.
   * Issue #453: Presigned IPFS Upload URLs — Bypass API for Large Files.
   *
   * @param walletAddress Stellar address for the keyName tag (audit trail only).
   */
  async generatePresignedKey(walletAddress: string): Promise<{
    uploadUrl: string;
    jwt: string;
    expiresIn: number;
    maxSize: number;
  }> {
    if (!this.pinataJwt) {
      throw new Error("PINATA_JWT is not configured");
    }

    const shortAddr = walletAddress.slice(0, 8);
    const response = await fetch("https://api.pinata.cloud/users/generateApiKey", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${this.pinataJwt}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        keyName: `upload-${shortAddr}-${Date.now()}`,
        permissions: { endpoints: { pinning: { pinFileToIPFS: true } } },
        maxUses: 1,
      }),
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`Failed to generate Pinata key (${response.status}): ${text}`);
    }

    const data = (await response.json()) as { JWT: string };
    return {
      uploadUrl: "https://api.pinata.cloud/pinning/pinFileToIPFS",
      jwt: data.JWT,
      expiresIn: 900,
      maxSize: 10 * 1024 * 1024,
    };
  }

  /**
   * Verifies that a CID is publicly accessible via a HEAD request against the
   * configured gateway. Returns `true` on HTTP 2xx, `false` otherwise.
   * Issue #453.
   */
  async verifyCid(cid: string): Promise<boolean> {
    if (!/^(Qm[1-9A-HJ-NP-Za-km-z]{44}|baf[a-z2-7]{56})$/.test(cid)) {
      return false;
    }
    try {
      const res = await fetch(`${this.gatewayBase}/ipfs/${cid}`, { method: "HEAD" });
      return res.ok;
    } catch {
      return false;
    }
  }
}

export const ipfsService = new IpfsService();
