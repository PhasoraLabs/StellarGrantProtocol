/**
 * Example: Upload and fetch grant metadata using IPFS
 * 
 * This example demonstrates:
 * - Uploading rich metadata to IPFS
 * - Fetching metadata with fallback gateways
 * - Integrating IPFS with grant creation
 */

import { StellarGrantsSDK, uploadMetadataToIPFS, fetchMetadataFromIPFS } from "../src";
import type { WalletAdapter } from "../src/types";

async function main() {
  const rpcUrl = process.env.RPC_URL;
  const networkPassphrase = process.env.NETWORK_PASSPHRASE;
  const contractId = process.env.CONTRACT_ID;
  const pinataJwt = process.env.PINATA_JWT;

  if (!rpcUrl || !networkPassphrase || !contractId || !pinataJwt) {
    throw new Error("Missing env: RPC_URL, NETWORK_PASSPHRASE, CONTRACT_ID, PINATA_JWT");
  }

  // Example 1: Upload metadata to IPFS
  console.log("Uploading metadata to IPFS...");
  
  const metadata = {
    title: "Ocean Cleanup Initiative",
    description: "A comprehensive program to clean coastal areas and reduce ocean pollution",
    longDescription: `
      This grant funds a multi-phase ocean cleanup initiative targeting coastal areas
      with high pollution levels. The project includes:
      
      - Beach cleanup operations
      - Marine debris collection
      - Community education programs
      - Recycling infrastructure development
      
      Expected impact: 50 tons of waste removed, 10,000 community members engaged.
    `,
    team: [
      { name: "Alice Johnson", role: "Project Lead", bio: "Marine biologist with 10 years experience" },
      { name: "Bob Smith", role: "Operations Manager", bio: "Environmental engineer" },
    ],
    milestones: [
      { title: "Phase 1: Planning", description: "Complete project planning and team assembly" },
      { title: "Phase 2: Equipment", description: "Acquire cleanup equipment and supplies" },
      { title: "Phase 3: Execution", description: "Conduct cleanup operations" },
      { title: "Phase 4: Reporting", description: "Document results and impact" },
    ],
    attachments: [
      "https://example.com/project-plan.pdf",
      "https://example.com/budget-breakdown.xlsx",
    ],
    tags: ["environment", "ocean", "cleanup", "sustainability"],
  };

  const { cid, gatewayUrl } = await uploadMetadataToIPFS(metadata, {
    pinataJwt,
    name: "ocean-cleanup-grant-metadata",
  });

  console.log(`✓ Metadata uploaded successfully!`);
  console.log(`  CID: ${cid}`);
  console.log(`  Gateway URL: ${gatewayUrl}`);
  console.log(`  IPFS URI: ipfs://${cid}`);

  // Example 2: Fetch metadata from IPFS
  console.log("\nFetching metadata from IPFS...");
  
  const fetchedMetadata = await fetchMetadataFromIPFS(cid, [
    "https://gateway.pinata.cloud/ipfs/",
    "https://ipfs.io/ipfs/",
  ]);

  console.log("✓ Metadata fetched successfully!");
  console.log("  Title:", fetchedMetadata.title);
  console.log("  Team members:", (fetchedMetadata.team as any[]).length);

  // Example 3: Create grant with IPFS metadata
  console.log("\nCreating grant with IPFS metadata...");

  const signer: WalletAdapter = {
    name: "Example Signer",
    isAvailable: () => true,
    async getPublicKey() {
      // TODO: Replace with real wallet integration
      return "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
    },
    async signTransaction(txXdr: string) {
      // TODO: Replace with real wallet signing
      return txXdr;
    },
  };

  const sdk = new StellarGrantsSDK({
    rpcUrl,
    networkPassphrase,
    contractId,
    signer,
  });

  const owner = await signer.getPublicKey();
  
  // Option A: Manual IPFS integration
  const resultManual = await sdk.grantCreate({
    owner,
    title: metadata.title,
    description: `ipfs://${cid}`, // Store IPFS CID as description
    budget: BigInt("5000000000"),
    deadline: BigInt(Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 90),
    milestoneCount: 4,
  });

  console.log("✓ Grant created with manual IPFS integration:", resultManual);

  // Option B: Automatic IPFS upload
  const resultAuto = await sdk.grantCreate(
    {
      owner,
      title: "Another Grant",
      description: "This description will be automatically uploaded to IPFS",
      budget: BigInt("3000000000"),
      deadline: BigInt(Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 60),
      milestoneCount: 3,
    },
    {
      uploadMetadata: true,
      ipfsConfig: { pinataJwt },
    }
  );

  console.log("✓ Grant created with automatic IPFS upload:", resultAuto);

  // Example 4: Fetch grant with IPFS metadata
  console.log("\nFetching grant with IPFS metadata...");
  
  const grant = await sdk.grantGet(1, {
    fetchIpfsMetadata: true,
    ipfsGateways: ["https://gateway.pinata.cloud/ipfs/"],
  });

  console.log("✓ Grant fetched with metadata:", grant);
}

main().catch((err) => {
  console.error("Error:", err);
  process.exit(1);
});
