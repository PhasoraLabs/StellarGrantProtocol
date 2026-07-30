use soroban_sdk::{contracttype, Address};

#[derive(Clone)]
#[contracttype]
pub enum StorageKey {
    GrantCounter,
    Grant(u64),
    BountyCounter,
    Bounty(u64),
    BountySubmission { bounty_id: u64, submitter: Address },
    BountySubmitters(u64),
    Metrics,
}
