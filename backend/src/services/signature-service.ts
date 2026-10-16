import { Keypair, StrKey } from "@stellar/stellar-sdk";

export type MilestoneProofIntent = {
  grantId: number;
  milestoneIdx: number;
  proofCid: string;
  submittedBy: string;
  nonce: string;
  timestamp: number;
  signature: string;
};

export type AdminIntent = {
  address: string;
  nonce: string;
  timestamp: number;
  action: string;
  signature: string;
};

export type FeedbackIntent = {
  grantId: number;
  rating: number;
  role: "funder" | "reviewer" | "recipient";
  address: string;
  nonce: string;
  timestamp: number;
  signature: string;
};

/** Signed intent for grant-owner actions such as adding a reviewer (#1238). */
export type AddReviewerIntent = {
  grantId: number;
  reviewerStellarAddress: string;
  address: string;
  nonce: string;
  timestamp: number;
  signature: string;
};

export type MilestoneApprovalIntent = {
  grantId: number;
  milestoneIdx: number;
  reviewerStellarAddress: string;
  approved: boolean;
  address: string;
  nonce: string;
  timestamp: number;
  signature: string;
};

export class SignatureService {
  buildIntentMessage(payload: Omit<MilestoneProofIntent, "signature">): string {
    return [
      "stellargrant:milestone_proof:v1",
      payload.grantId,
      payload.milestoneIdx,
      payload.proofCid,
      payload.submittedBy,
      payload.nonce,
      payload.timestamp,
    ].join("|");
  }

  buildAdminIntentMessage(payload: Omit<AdminIntent, "signature">): string {
    return [
      "stellargrant:admin:v1",
      payload.address,
      payload.nonce,
      payload.timestamp,
      payload.action,
    ].join("|");
  }

  buildFeedbackIntentMessage(payload: Omit<FeedbackIntent, "signature">): string {
    return [
      "stellargrant:feedback:v1",
      payload.grantId,
      payload.rating,
      payload.role,
      payload.address,
      payload.nonce,
      payload.timestamp,
    ].join("|");
  }

  buildAddReviewerIntentMessage(payload: Omit<AddReviewerIntent, "signature">): string {
    return [
      "stellargrant:add_reviewer:v1",
      payload.grantId,
      payload.reviewerStellarAddress,
      payload.address,
      payload.nonce,
      payload.timestamp,
    ].join("|");
  }

  buildMilestoneApprovalIntentMessage(
    payload: Omit<MilestoneApprovalIntent, "signature">,
  ): string {
    return [
      "stellargrant:milestone_approval:v1",
      payload.grantId,
      payload.milestoneIdx,
      payload.reviewerStellarAddress,
      payload.approved,
      payload.address,
      payload.nonce,
      payload.timestamp,
    ].join("|");
  }

  verifyMilestoneApproval(payload: MilestoneApprovalIntent): boolean {
    try {
      if (
        payload.address !== payload.reviewerStellarAddress ||
        !StrKey.isValidEd25519PublicKey(payload.address)
      ) {
        return false;
      }
      const keypair = Keypair.fromPublicKey(payload.address);
      const message = this.buildMilestoneApprovalIntentMessage(payload);
      return keypair.verify(
        Buffer.from(message, "utf8"),
        Buffer.from(payload.signature, "base64"),
      );
    } catch {
      return false;
    }
  }

  verifyAddReviewer(payload: AddReviewerIntent): boolean {
    if (!StrKey.isValidEd25519PublicKey(payload.address)) {
      return false;
    }
    const keypair = Keypair.fromPublicKey(payload.address);
    const message = this.buildAddReviewerIntentMessage(payload);
    return keypair.verify(
      Buffer.from(message, "utf8"),
      Buffer.from(payload.signature, "base64"),
    );
  }

  verify(payload: MilestoneProofIntent | FeedbackIntent): boolean {
    if ("address" in payload) {
      if (!StrKey.isValidEd25519PublicKey(payload.address)) {
        return false;
      }

      const keypair = Keypair.fromPublicKey(payload.address);
      const message = this.buildFeedbackIntentMessage(payload);

      return keypair.verify(
        Buffer.from(message, "utf8"),
        Buffer.from(payload.signature, "base64"),
      );
    } else {
      if (!StrKey.isValidEd25519PublicKey(payload.submittedBy)) {
        return false;
      }

      const keypair = Keypair.fromPublicKey(payload.submittedBy);
      const message = this.buildIntentMessage(payload);

      return keypair.verify(
        Buffer.from(message, "utf8"),
        Buffer.from(payload.signature, "base64"),
      );
    }
  }

  verifyAdmin(payload: AdminIntent): boolean {
    if (!StrKey.isValidEd25519PublicKey(payload.address)) {
      return false;
    }

    const keypair = Keypair.fromPublicKey(payload.address);
    const message = this.buildAdminIntentMessage(payload);

    return keypair.verify(
      Buffer.from(message, "utf8"),
      Buffer.from(payload.signature, "base64"),
    );
  }
}
