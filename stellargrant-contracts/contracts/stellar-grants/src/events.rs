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
}
