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
}
