use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub id: u64,
    pub owner: Address,
    pub title: String,
    pub description: String,
    pub token: Address,
    pub amount: i128,
    pub status: GrantStatus,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GrantStatus {
    Active,
    Completed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BountyGrant {
    pub id: u64,
    pub owner: Address,
    pub title: String,
    pub description: String,
    pub token: Address,
    pub prize_amount: i128,
    pub status: BountyStatus,
    pub submission_deadline: u64,
    pub winner: Option<Address>,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BountyStatus {
    Open,
    UnderReview,
    Awarded,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BountySubmission {
    pub bounty_id: u64,
    pub submitter: Address,
    pub proof_url: String,
    pub submitted_at: u64,
}
