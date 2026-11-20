use soroban_sdk::contracterror;

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
    Reentrancy = 25,
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
    ZeroAmount = 22,
    ReviewerLimitExceeded = 23,
    MilestoneIndexOutOfBounds = 24,
    ContractPaused = 26,
    // Streaming (#531)
    StreamNotFound = 27,
    StreamNotActive = 28,
    StreamAlreadyExists = 29,
    StreamExhausted = 30,
    // Quadratic Voting (#537)
    InsufficientVoiceCredits = 31,
    VoterNotAllocated = 32,
    // Insurance (#538)
    PolicyNotFound = 33,
    PolicyExpired = 34,
    PolicyInactive = 35,
    ClaimNotFound = 36,
    ClaimAlreadyResolved = 37,
    InsufficientPoolBalance = 38,
    // Hooks (#539)
    HookNotFound = 39,
    HookLimitExceeded = 40,
    HookAlreadyInactive = 41,
    // Escrow (#529)
    EscrowLocked = 42,
    EscrowAlreadyOpen = 43,
    EscrowNotFound = 44,
    // Multisig (#530)
    ProposalNotFound = 45,
    ProposalExpired = 46,
    ProposalAlreadyExecuted = 47,
    ThresholdNotMet = 48,
    NotAProposalSigner = 49,
    // Compliance (#548)
    ComplianceNotVerified = 50,
    ComplianceCheckFailed = 51,
    VerifierNotSet = 52,
    NotVerifier = 53,
    // Treasury (#519)
    InsufficientTreasuryBalance = 54,
    TreasuryNotConfigured = 55,
    // DAO Governance (#532)
    DaoProposalNotFound = 56,
    DaoProposalNotActive = 57,
    DaoProposalVotingClosed = 58,
    DaoProposalAlreadyExecuted = 59,
    DaoProposalQuorumNotReached = 60,
    DaoProposalRejected = 61,
    DaoModeDisabled = 62,
    // Bounty-Mode Grants (#533)
    BountyNotFound = 63,
    BountyNotOpen = 64,
    SubmissionWindowClosed = 65,
    SubmissionNotFound = 66,
    BountyAlreadyResolved = 67,
    NoSubmissions = 68,
    // Token Swap (#576)
    DexNotConfigured = 69,
    SwapExceedsSlippage = 70,
    SwapFailed = 71,
    InvalidSwapRoute = 72,
    // Checklist (#581)
    ChecklistNotFound = 73,
    CriterionNotFound = 74,
    ChecklistAlreadySubmitted = 75,
    RequiredCriteriaNotMet = 76,
    MaxCriteriaExceeded = 77,
    // Scoring (#589)
    RubricNotFound = 78,
    InvalidWeights = 79,
    // Circuit Breaker (#594)
    ModuleTripped = 80,
    BreakerNotTripped = 81,
    // Invoice (#566)
    InvoiceNotFound = 82,
    InvoiceAlreadySubmitted = 83,
    // Crowdfund module
    CrowdfundNotFound = 84,
    CrowdfundNotActive = 85,
    CrowdfundDeadlineNotReached = 86,
    CrowdfundAlreadyFinalized = 87,
    AlreadyPledged = 88,
    // Public review (#590)
    CommentTooLong = 89,
    ReviewNotFound = 90,
    // Milestone DAG (#595)
    DagAlreadyAttached = 91,
    DagCycleDetected = 92,
    DependencyNotSatisfied = 93,
    DagNotAttached = 94,
    // Milestone NFT (#570)
    NftNotFound = 95,
    NftNotTransferable = 96,
    NotNftOwner = 97,
    // Referral system (#569)
    ReferralCodeNotFound = 98,
    ReferralCodeInactive = 99,
    ReferralCodeExpired = 100,
    ReferralCodeExhausted = 101,
    AlreadyReferred = 102,
    ReferralRecordNotFound = 103,
    NoRewardsToClaim = 104,
    // Deadline extension (#572)
    ExtensionRequestNotFound = 105,
    ExtensionAlreadyResolved = 106,
    NoDeadlineSet = 107,
    // Arbitration pool (#573)
    ArbiterNotFound = 108,
    ArbiterAlreadyJoined = 109,
    ArbiterInActiveCase = 110,
    ArbitrationCaseNotFound = 111,
    CaseAlreadyFinalized = 112,
    CaseNotFinalized = 113,
    NotPanelMember = 114,
    VotingDeadlinePassed = 115,
    InsufficientArbiters = 116,
    // Performance bond (#574)
    BondNotFound = 117,
    BondAlreadyPosted = 118,
    BondNotPosted = 119,
    BondNotActive = 120,
    BondExpired = 121,
    // Collateral (#564)
    CollateralAlreadyDeposited = 122,
    CollateralNotDeposited = 123,
    // Whitelist (#512)
    AddressNotWhitelisted = 124,
    // KYC verification (#632)
    KycRequired = 131,
    // Math (#528) — no specific errors, reuses ZeroAmount / InvalidInput
    // Funder report (#598) — no specific errors, read-only
    // Batch read (#622)
    BatchSizeExceeded = 125,
    // Conditional release (#613)
    MaxConditionsExceeded = 126,
    ConditionCheckFailed = 127,
    // Auto-approve (#612)
    AutoApproveNotEnabled = 128,
    AutoApproveGracePeriodNotPassed = 129,
    AutoApproveInsufficientVotes = 130,
    // Grant timer (#618)
    TimerNotFound = 137,
    TimerAlreadyFired = 132,
    TimerNotEligible = 133,
    // Waitlist module
    WaitlistFull = 134,
    AlreadyOnWaitlist = 135,
    NotOnWaitlist = 136,
    ContributorNotFound = 138,
    // Lockup (#609)
    LockupAlreadyExists = 139,
    LockupNotFound = 140,
    LockupAlreadyReleased = 141,
    NotYetUnlocked = 142,
    LockupRevocationUnauthorized = 143,
    LockupAlreadyRevoked = 144,
    // Delegation (#907)
    DelegationChainTooLong = 145,
    // Clawback (#909)
    InsufficientClawbackAllowance = 146,
    // Token swap (#576) — stub
    SwapNotImplemented = 147,
    // Public review (#590) — limit guard
    TooManyPublicReviews = 148,
    // DAO vote gate
    DaoVoteRequired = 149,
    BountySubmissionLimitExceeded = 150,
    // RBAC bootstrap (#1077)
    AlreadyInitialized = 151,
}

// ─── Discriminant uniqueness tests ────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Maximum discriminant value used in the ContractError enum.
    /// Update this if a higher discriminant is added.
    const MAX_DISCRIMINANT: usize = 145;

    #[test]
    fn test_no_duplicate_discriminants() {
        let mut seen = [false; MAX_DISCRIMINANT + 1];
        let pairs: &[(ContractError, u32)] = &[
            (ContractError::GrantNotFound, 1),
            (ContractError::Unauthorized, 2),
            (ContractError::MilestoneAlreadyApproved, 3),
            (ContractError::QuorumNotReached, 4),
            (ContractError::DeadlinePassed, 5),
            (ContractError::InvalidInput, 6),
            (ContractError::MilestoneNotSubmitted, 7),
            (ContractError::AlreadyVoted, 8),
            (ContractError::MilestoneNotFound, 9),
            (ContractError::InvalidState, 10),
            (ContractError::NoRefundableAmount, 11),
            (ContractError::GrantAlreadyReleased, 12),
            (ContractError::NotMultisigSigner, 13),
            (ContractError::AlreadySignedRelease, 14),
            (ContractError::NotAllMilestonesApproved, 15),
            (ContractError::InsufficientStake, 16),
            (ContractError::StakeNotFound, 17),
            (ContractError::AlreadyRegistered, 18),
            (ContractError::BatchEmpty, 19),
            (ContractError::BatchTooLarge, 20),
            (ContractError::MilestoneAlreadySubmitted, 21),
            (ContractError::ZeroAmount, 22),
            (ContractError::ReviewerLimitExceeded, 23),
            (ContractError::MilestoneIndexOutOfBounds, 24),
            (ContractError::Reentrancy, 25),
            (ContractError::ContractPaused, 26),
            (ContractError::StreamNotFound, 27),
            (ContractError::StreamNotActive, 28),
            (ContractError::StreamAlreadyExists, 29),
            (ContractError::StreamExhausted, 30),
            (ContractError::InsufficientVoiceCredits, 31),
            (ContractError::VoterNotAllocated, 32),
            (ContractError::PolicyNotFound, 33),
            (ContractError::PolicyExpired, 34),
            (ContractError::PolicyInactive, 35),
            (ContractError::ClaimNotFound, 36),
            (ContractError::ClaimAlreadyResolved, 37),
            (ContractError::InsufficientPoolBalance, 38),
            (ContractError::HookNotFound, 39),
            (ContractError::HookLimitExceeded, 40),
            (ContractError::HookAlreadyInactive, 41),
            (ContractError::EscrowLocked, 42),
            (ContractError::EscrowAlreadyOpen, 43),
            (ContractError::EscrowNotFound, 44),
            (ContractError::ProposalNotFound, 45),
            (ContractError::ProposalExpired, 46),
            (ContractError::ProposalAlreadyExecuted, 47),
            (ContractError::ThresholdNotMet, 48),
            (ContractError::NotAProposalSigner, 49),
            (ContractError::ComplianceNotVerified, 50),
            (ContractError::ComplianceCheckFailed, 51),
            (ContractError::VerifierNotSet, 52),
            (ContractError::NotVerifier, 53),
            (ContractError::InsufficientTreasuryBalance, 54),
            (ContractError::TreasuryNotConfigured, 55),
            (ContractError::DaoProposalNotFound, 56),
            (ContractError::DaoProposalNotActive, 57),
            (ContractError::DaoProposalVotingClosed, 58),
            (ContractError::DaoProposalAlreadyExecuted, 59),
            (ContractError::DaoProposalQuorumNotReached, 60),
            (ContractError::DaoProposalRejected, 61),
            (ContractError::DaoModeDisabled, 62),
            (ContractError::BountyNotFound, 63),
            (ContractError::BountyNotOpen, 64),
            (ContractError::SubmissionWindowClosed, 65),
            (ContractError::SubmissionNotFound, 66),
            (ContractError::BountyAlreadyResolved, 67),
            (ContractError::NoSubmissions, 68),
            (ContractError::DexNotConfigured, 69),
            (ContractError::SwapExceedsSlippage, 70),
            (ContractError::SwapFailed, 71),
            (ContractError::InvalidSwapRoute, 72),
            (ContractError::ChecklistNotFound, 73),
            (ContractError::CriterionNotFound, 74),
            (ContractError::ChecklistAlreadySubmitted, 75),
            (ContractError::RequiredCriteriaNotMet, 76),
            (ContractError::MaxCriteriaExceeded, 77),
            (ContractError::RubricNotFound, 78),
            (ContractError::InvalidWeights, 79),
            (ContractError::ModuleTripped, 80),
            (ContractError::BreakerNotTripped, 81),
            (ContractError::InvoiceNotFound, 82),
            (ContractError::InvoiceAlreadySubmitted, 83),
            (ContractError::CrowdfundNotFound, 84),
            (ContractError::CrowdfundNotActive, 85),
            (ContractError::CrowdfundDeadlineNotReached, 86),
            (ContractError::CrowdfundAlreadyFinalized, 87),
            (ContractError::AlreadyPledged, 88),
            (ContractError::CommentTooLong, 89),
            (ContractError::ReviewNotFound, 90),
            (ContractError::DagAlreadyAttached, 91),
            (ContractError::DagCycleDetected, 92),
            (ContractError::DependencyNotSatisfied, 93),
            (ContractError::DagNotAttached, 94),
            (ContractError::NftNotFound, 95),
            (ContractError::NftNotTransferable, 96),
            (ContractError::NotNftOwner, 97),
            (ContractError::ReferralCodeNotFound, 98),
            (ContractError::ReferralCodeInactive, 99),
            (ContractError::ReferralCodeExpired, 100),
            (ContractError::ReferralCodeExhausted, 101),
            (ContractError::AlreadyReferred, 102),
            (ContractError::ReferralRecordNotFound, 103),
            (ContractError::NoRewardsToClaim, 104),
            (ContractError::ExtensionRequestNotFound, 105),
            (ContractError::ExtensionAlreadyResolved, 106),
            (ContractError::NoDeadlineSet, 107),
            (ContractError::ArbiterNotFound, 108),
            (ContractError::ArbiterAlreadyJoined, 109),
            (ContractError::ArbiterInActiveCase, 110),
            (ContractError::ArbitrationCaseNotFound, 111),
            (ContractError::CaseAlreadyFinalized, 112),
            (ContractError::CaseNotFinalized, 113),
            (ContractError::NotPanelMember, 114),
            (ContractError::VotingDeadlinePassed, 115),
            (ContractError::InsufficientArbiters, 116),
            (ContractError::BondNotFound, 117),
            (ContractError::BondAlreadyPosted, 118),
            (ContractError::BondNotPosted, 119),
            (ContractError::BondNotActive, 120),
            (ContractError::BondExpired, 121),
            (ContractError::CollateralAlreadyDeposited, 122),
            (ContractError::CollateralNotDeposited, 123),
            (ContractError::AddressNotWhitelisted, 124),
            (ContractError::BatchSizeExceeded, 125),
            (ContractError::MaxConditionsExceeded, 126),
            (ContractError::ConditionCheckFailed, 127),
            (ContractError::AutoApproveNotEnabled, 128),
            (ContractError::AutoApproveGracePeriodNotPassed, 129),
            (ContractError::AutoApproveInsufficientVotes, 130),
            (ContractError::KycRequired, 131),
            (ContractError::TimerAlreadyFired, 132),
            (ContractError::TimerNotEligible, 133),
            (ContractError::WaitlistFull, 134),
            (ContractError::AlreadyOnWaitlist, 135),
            (ContractError::NotOnWaitlist, 136),
            (ContractError::TimerNotFound, 137),
            (ContractError::ContributorNotFound, 138),
            (ContractError::LockupAlreadyExists, 139),
            (ContractError::LockupNotFound, 140),
            (ContractError::LockupAlreadyReleased, 141),
            (ContractError::NotYetUnlocked, 142),
            (ContractError::LockupRevocationUnauthorized, 143),
            (ContractError::LockupAlreadyRevoked, 144),
            (ContractError::DaoVoteRequired, 145),
        ];
        for (_, disc) in pairs {
            let idx = *disc as usize;
            assert!(
                idx <= MAX_DISCRIMINANT,
                "Discriminant {} exceeds MAX_DISCRIMINANT",
                disc
            );
            assert!(
                !seen[idx],
                "Duplicate discriminant {} found in ContractError",
                disc
            );
            seen[idx] = true;
        }
        let mut count = 0;
        for &v in &seen {
            if v {
                count += 1;
            }
        }
        assert_eq!(
            count,
            pairs.len(),
            "Number of unique discriminants must equal number of variants"
        );
    }
}
