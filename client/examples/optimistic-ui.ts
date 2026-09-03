/**
 * Example: Implement optimistic UI updates
 * 
 * This example demonstrates:
 * - Using TransactionTracker for event-driven updates
 * - Managing optimistic state with OptimisticStateManager
 * - Handling transaction failures with rollback
 * - Building responsive UIs during blockchain operations
 */

import { 
  StellarGrantsSDK, 
  TransactionTracker, 
  OptimisticStateManager 
} from "../src";
import type { WalletAdapter } from "../src/types";

// Simulated UI state
let grants: any[] = [];
let notifications: string[] = [];

function updateUI(newGrants: any[]) {
  grants = newGrants;
  console.log("UI Updated - Grants:", grants.length);
}

function showNotification(message: string) {
  notifications.push(message);
  console.log(`[Notification] ${message}`);
}

function showError(message: string) {
  console.error(`[Error] ${message}`);
}

async function main() {
  const rpcUrl = process.env.RPC_URL;
  const networkPassphrase = process.env.NETWORK_PASSPHRASE;
  const contractId = process.env.CONTRACT_ID;

  if (!rpcUrl || !networkPassphrase || !contractId) {
    throw new Error("Missing env: RPC_URL, NETWORK_PASSPHRASE, CONTRACT_ID");
  }

  const signer: WalletAdapter = {
    name: "Example Signer",
    isAvailable: () => true,
    async getPublicKey() {
      return "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
    },
    async signTransaction(txXdr: string) {
      return txXdr;
    },
  };

  const sdk = new StellarGrantsSDK({
    rpcUrl,
    networkPassphrase,
    contractId,
    signer,
  });

  // Initialize optimistic state manager and transaction tracker
  const stateManager = new OptimisticStateManager();
  const tracker = new TransactionTracker();

  // Set up transaction event listeners
  tracker.on("signed", (txId) => {
    showNotification("Transaction signed successfully");
    console.log(`Transaction ${txId} signed`);
  });

  tracker.on("submitted", (txId, hash) => {
    showNotification(`Transaction submitted: ${hash.substring(0, 8)}...`);
    console.log(`Transaction ${txId} submitted with hash ${hash}`);
  });

  tracker.on("confirmed", (txId, result) => {
    showNotification("Transaction confirmed!");
    console.log(`Transaction ${txId} confirmed:`, result);
    
    // Commit optimistic update
    const actualState = result; // In real app, parse result properly
    stateManager.commit(txId, actualState);
  });

  tracker.on("failed", (txId, error) => {
    showError(`Transaction failed: ${error.message}`);
    console.error(`Transaction ${txId} failed:`, error);
    
    // Rollback optimistic update
    const previousState = stateManager.rollback(txId);
    if (previousState) {
      updateUI(previousState);
      showNotification("Changes reverted");
    }
  });

  tracker.on("stageChange", (txId, stage) => {
    console.log(`Transaction ${txId} stage changed to: ${stage}`);
  });

  // Example 1: Optimistic grant creation
  console.log("\n=== Example 1: Optimistic Grant Creation ===");
  
  const owner = await signer.getPublicKey();
  const grantInput = {
    owner,
    title: "Community Garden Project",
    description: "Building sustainable urban gardens",
    budget: BigInt("2000000000"),
    deadline: BigInt(Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 45),
    milestoneCount: 3,
  };

  // Predict the state after grant creation
  const predictedGrant = stateManager.predictGrantCreate(grantInput);
  console.log("Predicted grant state:", predictedGrant);

  // Apply optimistic update immediately
  const txId = `tx_${Date.now()}`;
  stateManager.apply(txId, predictedGrant, "grant_create", grants);
  
  // Update UI with optimistic state
  updateUI([...grants, predictedGrant]);
  showNotification("Creating grant...");

  try {
    // Track the actual transaction
    await tracker.track(async () => {
      return await sdk.grantCreate(grantInput);
    }, predictedGrant);

    console.log("✓ Grant created successfully with optimistic UI");
  } catch (error) {
    console.error("✗ Grant creation failed, UI rolled back");
  }

  // Example 2: Optimistic grant funding
  console.log("\n=== Example 2: Optimistic Grant Funding ===");

  const currentGrant = {
    id: 1,
    owner,
    title: "Existing Grant",
    description: "Some grant",
    budget: BigInt("5000000000"),
    raised: BigInt("2000000000"),
    deadline: BigInt(Date.now() / 1000 + 86400 * 30),
    milestoneCount: 4,
    status: "pending" as const,
    createdAt: Date.now(),
  };

  const fundInput = {
    grantId: 1,
    token: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
    amount: BigInt("1000000000"),
  };

  // Predict state after funding
  const predictedFundedGrant = stateManager.predictGrantFund(currentGrant, fundInput);
  console.log("Predicted funded grant:", {
    raised: predictedFundedGrant.raised.toString(),
    status: predictedFundedGrant.status,
  });

  const fundTxId = `tx_fund_${Date.now()}`;
  stateManager.apply(fundTxId, predictedFundedGrant, "grant_fund", currentGrant);
  
  showNotification("Funding grant...");

  try {
    await tracker.track(async () => {
      return await sdk.grantFund(fundInput);
    }, predictedFundedGrant);

    console.log("✓ Grant funded successfully with optimistic UI");
  } catch (error) {
    console.error("✗ Grant funding failed, UI rolled back");
  }

  // Example 3: Optimistic milestone submission
  console.log("\n=== Example 3: Optimistic Milestone Submission ===");

  const currentMilestone = {
    grantId: 1,
    index: 0,
    title: "Phase 1",
    description: "Initial setup",
    amount: BigInt("1000000000"),
    status: "pending" as const,
  };

  const proofHash = "QmX7Y8Z9..."; // IPFS CID

  // Predict state after submission
  const predictedMilestone = stateManager.predictMilestoneSubmit(
    currentMilestone,
    proofHash
  );
  console.log("Predicted milestone state:", predictedMilestone);

  const milestoneTxId = `tx_milestone_${Date.now()}`;
  stateManager.apply(milestoneTxId, predictedMilestone, "milestone_submit", currentMilestone);
  
  showNotification("Submitting milestone proof...");

  try {
    await tracker.track(async () => {
      return await sdk.milestoneSubmit({
        grantId: 1,
        milestoneIdx: 0,
        proofHash,
      });
    }, predictedMilestone);

    console.log("✓ Milestone submitted successfully with optimistic UI");
  } catch (error) {
    console.error("✗ Milestone submission failed, UI rolled back");
  }

  // Example 4: Optimistic milestone voting
  console.log("\n=== Example 4: Optimistic Milestone Voting ===");

  const submittedMilestone = {
    ...currentMilestone,
    status: "submitted" as const,
    submittedAt: Date.now(),
  };

  const currentVotes = { approve: 2, reject: 0 };
  const threshold = 3;

  // Predict state after vote
  const predictedVotedMilestone = stateManager.predictMilestoneVote(
    submittedMilestone,
    true, // approve
    currentVotes,
    threshold
  );
  console.log("Predicted milestone after vote:", predictedVotedMilestone);

  const voteTxId = `tx_vote_${Date.now()}`;
  stateManager.apply(voteTxId, predictedVotedMilestone, "milestone_vote", submittedMilestone);
  
  showNotification("Casting vote...");

  try {
    await tracker.track(async () => {
      return await sdk.milestoneVote({
        grantId: 1,
        milestoneIdx: 0,
        approve: true,
      });
    }, predictedVotedMilestone);

    console.log("✓ Vote cast successfully with optimistic UI");
  } catch (error) {
    console.error("✗ Vote failed, UI rolled back");
  }

  // Clean up old operations
  stateManager.clearOld(300000); // Clear operations older than 5 minutes

  console.log("\n=== Summary ===");
  console.log("Pending operations:", stateManager.getPendingOperations().length);
  console.log("Tracked transactions:", tracker.getAllTransactions().length);
  console.log("Notifications:", notifications.length);
}

main().catch((err) => {
  console.error("Error:", err);
  process.exit(1);
});
