use crate::types::MilestoneState;
use soroban_sdk::{contractevent, Address, Env, String};

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantCancelled {
    pub grant_id: u64,
    pub owner: Address,
    pub reason: String,
    pub refund_amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefundExecuted {
    pub grant_id: u64,
    pub funder: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefundIssued {
    pub grant_id: u64,
    pub funder: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantCompleted {
    pub grant_id: u64,
    pub total_paid: i128,
    pub remaining_balance: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinalRefund {
    pub grant_id: u64,
    pub funder: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContributorRegistered {
    pub contributor: Address,
    pub name: String,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneSubmitted {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub description: String,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantFunded {
    pub grant_id: u64,
    pub funder: Address,
    pub amount: i128,
    pub new_balance: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantCreated {
    pub grant_id: u64,
    pub owner: Address,
    pub title: String,
    pub total_amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneVoted {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub reviewer: Address,
    pub approve: bool,
    pub feedback: Option<String>,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneRejected {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub reviewer: Address,
    pub reason: String,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneStatusChanged {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub new_state: MilestoneState,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestonePaid {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractMigrated {
    pub from_version: u32,
    pub to_version: u32,
    pub run_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewerApproved {
    pub reviewer: Address,
    pub approved_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewerRevoked {
    pub reviewer: Address,
    pub revoked_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractPaused {
    pub admin: Address,
    pub reason: String,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractUnpaused {
    pub admin: Address,
    pub timestamp: u64,
}

// ── Issue #514: Dispute events ────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisputeRaised {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub raised_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArbiterAssigned {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub arbiter: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArbiterVoted {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub arbiter: Address,
    pub favor_contributor: bool,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisputeResolved {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub resolved_for_contributor: bool,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisputeCancelled {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub cancelled_by: Address,
    pub timestamp: u64,
}

// ── Issue #515: Reputation event ──────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReputationUpdated {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub contributor: Address,
    pub new_reputation_score: u64,
    pub total_earned: i128,
    pub timestamp: u64,
}

// ── Issue #517: Fee collected event ──────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeeCollected {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub fee_amount: i128,
    pub token: Address,
    pub treasury: Address,
    pub timestamp: u64,
}

// ── Issue #519: Treasury events ───────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreasuryDeposited {
    pub token: Address,
    pub from: Address,
    pub amount: i128,
    pub new_balance: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreasuryWithdrawn {
    pub token: Address,
    pub to: Address,
    pub amount: i128,
    pub new_balance: i128,
    pub admin: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreasuryReallocated {
    pub from_token: Address,
    pub to_token: Address,
    pub amount: i128,
    pub admin: Address,
    pub timestamp: u64,
}

// ── Issue #532: DAO governance events ─────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaoProposalCreated {
    pub proposal_id: u64,
    pub proposer: Address,
    pub title: String,
    pub voting_deadline: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaoVoteCast {
    pub proposal_id: u64,
    pub voter: Address,
    pub support: bool,
    pub weight: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaoProposalFinalized {
    pub proposal_id: u64,
    pub passed: bool,
    pub votes_for: u64,
    pub votes_against: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaoProposalExecuted {
    pub proposal_id: u64,
    pub executed_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaoProposalCancelled {
    pub proposal_id: u64,
    pub cancelled_by: Address,
    pub timestamp: u64,
}

// ── Issue #533: Bounty-mode grant events ──────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BountyCreated {
    pub bounty_id: u64,
    pub owner: Address,
    pub title: String,
    pub prize_amount: i128,
    pub submission_deadline: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BountySubmissionReceived {
    pub bounty_id: u64,
    pub submitter: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BountyAwarded {
    pub bounty_id: u64,
    pub winner: Address,
    pub prize_amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BountyCancelled {
    pub bounty_id: u64,
    pub cancelled_by: Address,
    pub refund_amount: i128,
    pub timestamp: u64,
}

pub struct Events;

impl Events {
    pub fn emit_grant_cancelled(
        env: &Env,
        grant_id: u64,
        owner: Address,
        reason: String,
        refund_amount: i128,
    ) {
        let event = GrantCancelled {
            grant_id,
            owner,
            reason,
            refund_amount,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_refund_executed(env: &Env, grant_id: u64, funder: Address, amount: i128) {
        let event = RefundExecuted {
            grant_id,
            funder,
            amount,
        };
        event.publish(env);
    }

    pub fn emit_refund_issued(env: &Env, grant_id: u64, funder: Address, amount: i128) {
        let event = RefundIssued {
            grant_id,
            funder,
            amount,
        };
        event.publish(env);
    }

    pub fn emit_grant_completed(
        env: &Env,
        grant_id: u64,
        total_paid: i128,
        remaining_balance: i128,
    ) {
        let event = GrantCompleted {
            grant_id,
            total_paid,
            remaining_balance,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_final_refund(env: &Env, grant_id: u64, funder: Address, amount: i128) {
        let event = FinalRefund {
            grant_id,
            funder,
            amount,
        };
        event.publish(env);
    }

    pub fn emit_milestone_submitted(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        description: String,
    ) {
        let event = MilestoneSubmitted {
            grant_id,
            milestone_idx,
            description,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_grant_funded(
        env: &Env,
        grant_id: u64,
        funder: Address,
        amount: i128,
        new_balance: i128,
    ) {
        let event = GrantFunded {
            grant_id,
            funder,
            amount,
            new_balance,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_grant_created(
        env: &Env,
        grant_id: u64,
        owner: Address,
        title: String,
        total_amount: i128,
    ) {
        let event = GrantCreated {
            grant_id,
            owner,
            title,
            total_amount,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_contributor_registered(env: &Env, contributor: Address, name: String) {
        let event = ContributorRegistered {
            contributor,
            name,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn milestone_voted(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        reviewer: Address,
        approve: bool,
        feedback: Option<String>,
    ) {
        let event = MilestoneVoted {
            grant_id,
            milestone_idx,
            reviewer,
            approve,
            feedback,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn milestone_rejected(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        reviewer: Address,
        reason: String,
    ) {
        let event = MilestoneRejected {
            grant_id,
            milestone_idx,
            reviewer,
            reason,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn milestone_status_changed(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        new_state: MilestoneState,
    ) {
        let event = MilestoneStatusChanged {
            grant_id,
            milestone_idx,
            new_state,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_milestone_paid(env: &Env, grant_id: u64, milestone_idx: u32, amount: i128) {
        let event = MilestonePaid {
            grant_id,
            milestone_idx,
            amount,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_contract_migrated(env: &Env, from_version: u32, to_version: u32, run_by: Address) {
        let event = ContractMigrated {
            from_version,
            to_version,
            run_by,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_reviewer_approved(env: &Env, reviewer: Address, approved_by: Address) {
        let event = ReviewerApproved {
            reviewer,
            approved_by,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_reviewer_revoked(env: &Env, reviewer: Address, revoked_by: Address) {
        let event = ReviewerRevoked {
            reviewer,
            revoked_by,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_contract_paused(env: &Env, admin: Address, reason: String) {
        let event = ContractPaused {
            admin,
            reason,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_contract_unpaused(env: &Env, admin: Address) {
        let event = ContractUnpaused {
            admin,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    // ── Issue #514: Dispute emit methods ──────────────────────────────────────

    pub fn emit_dispute_raised(env: &Env, grant_id: u64, milestone_idx: u32, raised_by: Address) {
        let event = DisputeRaised {
            grant_id,
            milestone_idx,
            raised_by,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_arbiter_assigned(env: &Env, grant_id: u64, milestone_idx: u32, arbiter: Address) {
        let event = ArbiterAssigned {
            grant_id,
            milestone_idx,
            arbiter,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_arbiter_voted(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        arbiter: Address,
        favor_contributor: bool,
    ) {
        let event = ArbiterVoted {
            grant_id,
            milestone_idx,
            arbiter,
            favor_contributor,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_dispute_resolved(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        resolved_for_contributor: bool,
    ) {
        let event = DisputeResolved {
            grant_id,
            milestone_idx,
            resolved_for_contributor,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_dispute_cancelled(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        cancelled_by: Address,
    ) {
        let event = DisputeCancelled {
            grant_id,
            milestone_idx,
            cancelled_by,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    // ── Issue #515: Reputation emit method ───────────────────────────────────

    pub fn emit_reputation_updated(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        contributor: Address,
        new_reputation_score: u64,
        total_earned: i128,
    ) {
        let event = ReputationUpdated {
            grant_id,
            milestone_idx,
            contributor,
            new_reputation_score,
            total_earned,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    // ── Issue #517: Fee emit method ───────────────────────────────────────────

    pub fn emit_fee_collected(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        fee_amount: i128,
        token: Address,
        treasury: Address,
    ) {
        let event = FeeCollected {
            grant_id,
            milestone_idx,
            fee_amount,
            token,
            treasury,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    // ── Issue #519: Treasury emit methods ─────────────────────────────────────

    pub fn emit_treasury_deposited(
        env: &Env,
        token: Address,
        from: Address,
        amount: i128,
        new_balance: i128,
    ) {
        let event = TreasuryDeposited {
            token,
            from,
            amount,
            new_balance,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_treasury_withdrawn(
        env: &Env,
        token: Address,
        to: Address,
        amount: i128,
        new_balance: i128,
        admin: Address,
    ) {
        let event = TreasuryWithdrawn {
            token,
            to,
            amount,
            new_balance,
            admin,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_treasury_reallocated(
        env: &Env,
        from_token: Address,
        to_token: Address,
        amount: i128,
        admin: Address,
    ) {
        let event = TreasuryReallocated {
            from_token,
            to_token,
            amount,
            admin,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    // ── Issue #532: DAO governance emit methods ───────────────────────────────

    pub fn emit_dao_proposal_created(
        env: &Env,
        proposal_id: u64,
        proposer: Address,
        title: String,
        voting_deadline: u64,
    ) {
        let event = DaoProposalCreated {
            proposal_id,
            proposer,
            title,
            voting_deadline,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_dao_vote_cast(
        env: &Env,
        proposal_id: u64,
        voter: Address,
        support: bool,
        weight: u64,
    ) {
        let event = DaoVoteCast {
            proposal_id,
            voter,
            support,
            weight,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_dao_proposal_finalized(
        env: &Env,
        proposal_id: u64,
        passed: bool,
        votes_for: u64,
        votes_against: u64,
    ) {
        let event = DaoProposalFinalized {
            proposal_id,
            passed,
            votes_for,
            votes_against,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_dao_proposal_executed(env: &Env, proposal_id: u64, executed_by: Address) {
        let event = DaoProposalExecuted {
            proposal_id,
            executed_by,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_dao_proposal_cancelled(env: &Env, proposal_id: u64, cancelled_by: Address) {
        let event = DaoProposalCancelled {
            proposal_id,
            cancelled_by,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    // ── Issue #533: Bounty emit methods ────────────────────────────────────────

    pub fn emit_bounty_created(
        env: &Env,
        bounty_id: u64,
        owner: Address,
        title: String,
        prize_amount: i128,
        submission_deadline: u64,
    ) {
        let event = BountyCreated {
            bounty_id,
            owner,
            title,
            prize_amount,
            submission_deadline,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_bounty_submission_received(env: &Env, bounty_id: u64, submitter: Address) {
        let event = BountySubmissionReceived {
            bounty_id,
            submitter,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_bounty_awarded(env: &Env, bounty_id: u64, winner: Address, prize_amount: i128) {
        let event = BountyAwarded {
            bounty_id,
            winner,
            prize_amount,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_bounty_cancelled(
        env: &Env,
        bounty_id: u64,
        cancelled_by: Address,
        refund_amount: i128,
    ) {
        let event = BountyCancelled {
            bounty_id,
            cancelled_by,
            refund_amount,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }
}

// ── Issue #530: Multisig events ───────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultisigProposalCreated {
    pub proposal_id: u32,
    pub grant_id: u64,
    pub created_by: Address,
    pub threshold: u32,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultisigSigned {
    pub proposal_id: u32,
    pub signer: Address,
    pub approved: bool,
    pub total_weight_signed: u32,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultisigExecuted {
    pub proposal_id: u32,
    pub grant_id: u64,
    pub executed_by: Address,
    pub timestamp: u64,
}

// ── Issue #548: Compliance events ─────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComplianceAttested {
    pub subject: Address,
    pub attested_by: Address,
    pub level: u32,
    pub expires_at: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComplianceRevoked {
    pub subject: Address,
    pub revoked_by: Address,
    pub timestamp: u64,
}

// ── Issue #566: Invoice events ────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvoiceSubmitted {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub invoice_number: String,
    pub total: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvoiceApproved {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub approved_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvoiceRejected {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub rejected_by: Address,
    pub reason: String,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvoiceResubmitted {
    pub grant_id: u64,
    pub milestone_idx: u32,
    pub total: i128,
    pub timestamp: u64,
}

// ── Issue #593: RBAC events ───────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleGranted {
    pub holder: Address,
    pub role: u32,
    pub granted_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleRevoked {
    pub holder: Address,
    pub role: u32,
    pub revoked_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleRenounced {
    pub holder: Address,
    pub role: u32,
    pub timestamp: u64,
}

impl Events {
    // ... existing methods ...

    // ── Invoice event emitters ────────────────────────────────────────────────

    pub fn invoice_submitted(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        invoice_number: String,
        total: i128,
    ) {
        let event = InvoiceSubmitted {
            grant_id,
            milestone_idx,
            invoice_number,
            total,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn invoice_approved(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        approved_by: Address,
    ) {
        let event = InvoiceApproved {
            grant_id,
            milestone_idx,
            approved_by,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn invoice_rejected(
        env: &Env,
        grant_id: u64,
        milestone_idx: u32,
        rejected_by: Address,
        reason: String,
    ) {
        let event = InvoiceRejected {
            grant_id,
            milestone_idx,
            rejected_by,
            reason,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn invoice_resubmitted(env: &Env, grant_id: u64, milestone_idx: u32, total: i128) {
        let event = InvoiceResubmitted {
            grant_id,
            milestone_idx,
            total,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    // ── RBAC event emitters ───────────────────────────────────────────────────

    pub fn role_granted(env: &Env, holder: Address, role: crate::types::Role, granted_by: Address) {
        let event = RoleGranted {
            holder,
            role: role as u32,
            granted_by,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn role_revoked(env: &Env, holder: Address, role: crate::types::Role, revoked_by: Address) {
        let event = RoleRevoked {
            holder,
            role: role as u32,
            revoked_by,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn role_renounced(env: &Env, holder: Address, role: crate::types::Role) {
        let event = RoleRenounced {
            holder,
            role: role as u32,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    // ── Crowdfund emit methods ────────────────────────────────────────────────

    pub fn emit_crowdfund_created(
        env: &Env,
        campaign_id: u64,
        owner: Address,
        title: String,
        target_amount: i128,
        deadline: u64,
    ) {
        let event = CrowdfundCreated {
            campaign_id,
            owner,
            title,
            target_amount,
            deadline,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_crowdfund_pledged(
        env: &Env,
        campaign_id: u64,
        backer: Address,
        amount: i128,
        total_pledged: i128,
    ) {
        let event = CrowdfundPledged {
            campaign_id,
            backer,
            amount,
            total_pledged,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_crowdfund_succeeded(env: &Env, campaign_id: u64, total_pledged: i128) {
        let event = CrowdfundSucceeded {
            campaign_id,
            total_pledged,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_crowdfund_failed(env: &Env, campaign_id: u64, total_pledged: i128) {
        let event = CrowdfundFailed {
            campaign_id,
            total_pledged,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_crowdfund_refunded(env: &Env, campaign_id: u64, backer: Address, amount: i128) {
        let event = CrowdfundRefunded {
            campaign_id,
            backer,
            amount,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }

    pub fn emit_crowdfund_cancelled(
        env: &Env,
        campaign_id: u64,
        cancelled_by: Address,
        total_pledged: i128,
    ) {
        let event = CrowdfundCancelled {
            campaign_id,
            cancelled_by,
            total_pledged,
            timestamp: env.ledger().timestamp(),
        };
        event.publish(env);
    }
}

// ── Crowdfund events ──────────────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrowdfundCreated {
    pub campaign_id: u64,
    pub owner: Address,
    pub title: String,
    pub target_amount: i128,
    pub deadline: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrowdfundPledged {
    pub campaign_id: u64,
    pub backer: Address,
    pub amount: i128,
    pub total_pledged: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrowdfundSucceeded {
    pub campaign_id: u64,
    pub total_pledged: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrowdfundFailed {
    pub campaign_id: u64,
    pub total_pledged: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrowdfundRefunded {
    pub campaign_id: u64,
    pub backer: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrowdfundCancelled {
    pub campaign_id: u64,
    pub cancelled_by: Address,
    pub total_pledged: i128,
    pub timestamp: u64,
}
