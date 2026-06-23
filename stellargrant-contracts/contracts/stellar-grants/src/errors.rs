use soroban_sdk::{contracterror, String};

/// Contract error types
#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ContractError {
    GrantNotFound = 1,
    Unauthorized = 2,
    MilestoneAlreadyApproved = 3,
    QuorumNotReached = 4,
    DeadlinePassed = 5,
    InvalidInput = 6,
    MilestoneNotSubmitted = 7,
    AlreadyVoted = 8,
    MilestoneNotFound = 9,
    InvalidState = 10,
    NoRefundableAmount = 11,
    GrantAlreadyReleased = 12,
    NotMultisigSigner = 13,
    AlreadySignedRelease = 14,
    NotAllMilestonesApproved = 15,
    InsufficientStake = 16,
    StakeNotFound = 17,
    AlreadyRegistered = 18,
    BatchEmpty = 19,
    BatchTooLarge = 20,
    MilestoneAlreadySubmitted = 21,
    ReviewerLimitExceeded = 22,
    MilestoneIndexOutOfBounds = 23,
    TokenMismatch = 24,
    ZeroAmount = 25,
    ContractPaused = 26,
}

/// Internal error context for structured logging (not a contract type).
#[allow(dead_code)]
pub struct ErrorContext {
    pub code: ContractError,
    pub grant_id: Option<u64>,
    pub detail: Option<String>,
}

#[allow(dead_code)]
pub fn with_context(err: ContractError, grant_id: u64) -> ErrorContext {
    ErrorContext {
        code: err,
        grant_id: Some(grant_id),
        detail: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_context_sets_grant_id_and_code() {
        let ctx = with_context(ContractError::GrantNotFound, 42);
        assert_eq!(ctx.code, ContractError::GrantNotFound);
        assert_eq!(ctx.grant_id, Some(42));
        assert_eq!(ctx.detail, None);
    }
}
