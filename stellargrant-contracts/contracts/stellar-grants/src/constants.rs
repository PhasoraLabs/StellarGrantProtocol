#![allow(dead_code)]

// ── Time ─────────────────────────────────────────────────────────────────────
pub const SECONDS_PER_DAY: u64 = 86_400;
pub const SECONDS_PER_WEEK: u64 = 604_800;
pub const SECONDS_PER_YEAR: u64 = 31_536_000;

// ── Financial ────────────────────────────────────────────────────────────────
pub const BASIS_POINTS_SCALE: u32 = 10_000;
pub const DEFAULT_PROTOCOL_FEE_BPS: u32 = 100; // 1%
pub const MAX_PROTOCOL_FEE_BPS: u32 = 1_000; // 10% ceiling
pub const MIN_GRANT_AMOUNT: i128 = 1_000_000; // 1 XLM in stroops
pub const MAX_GRANT_AMOUNT: i128 = 1_000_000_000_000; // 1M XLM ceiling

// ── Governance ───────────────────────────────────────────────────────────────
pub const DEFAULT_QUORUM_THRESHOLD_BPS: u32 = 5_001; // >50%
pub const MAX_REVIEWERS_PER_GRANT: u32 = 10;
pub const MIN_REVIEWERS_PER_GRANT: u32 = 1;

// ── Timelock / Ledger Timing ─────────────────────────────────────────────────
pub const LEDGERS_PER_SECOND: u64 = 1; // Stellar avg ~5s; use 1 for conservative
pub const LEDGERS_PER_HOUR: u32 = 720;
pub const LEDGERS_PER_DAY: u32 = 17_280;
pub const DEFAULT_TIMELOCK_DELAY_LEDGERS: u32 = 34_560; // 48 hours
pub const DEFAULT_DISPUTE_WINDOW_LEDGERS: u32 = 17_280; // 24 hours

// ── Lockup ───────────────────────────────────────────────────────────────────
pub const DEFAULT_LOCKUP_DURATION_SECONDS: u64 = SECONDS_PER_DAY * 90;

// ── SLA ──────────────────────────────────────────────────────────────────────
pub const DEFAULT_REVIEWER_SLA_SECONDS: u64 = SECONDS_PER_WEEK;
pub const DEFAULT_AUTO_APPROVE_GRACE_SECONDS: u64 = SECONDS_PER_DAY * 3;

// ── Clawback ─────────────────────────────────────────────────────────────────
pub const CLAWBACK_DISPUTE_WINDOW_SECONDS: u64 = SECONDS_PER_WEEK;

// ── String Limits ─────────────────────────────────────────────────────────────
pub const MAX_TITLE_LEN: u32 = 128;
pub const MAX_DESCRIPTION_LEN: u32 = 1_024;
pub const MAX_PROOF_URL_LEN: u32 = 512;
pub const MAX_BIO_LEN: u32 = 256;

// ── Pagination ────────────────────────────────────────────────────────────────
pub const MAX_PAGE_SIZE: u32 = 50;
pub const DEFAULT_PAGE_SIZE: u32 = 20;
pub const MAX_EXPORT_PAGE_SIZE: u32 = 50;

// ── Reputation ────────────────────────────────────────────────────────────────
pub const MAX_REPUTATION_SCORE: u32 = 1_000;
pub const REPUTATION_REJECTION_PENALTY: u32 = 200; // bps subtracted per rejection

// ── Milestones ────────────────────────────────────────────────────────────────
pub const MAX_MILESTONES_PER_GRANT: u32 = 20;
pub const MAX_BATCH_SIZE: u32 = 10;

// ── Limits ───────────────────────────────────────────────────────────────────
pub const MAX_FORK_DEPTH: u32 = 5;
pub const MAX_SPLIT_RECIPIENTS: u32 = 10;
pub const MAX_CRITERIA_PER_MILESTONE: u32 = 20;
pub const MAX_INDEX_ENTRIES: u32 = 10_000;
pub const MAX_PUBLIC_REVIEW_COMMENT_LEN: u32 = 500;
pub const MAX_ROLLING_WINDOW_SIZE: u32 = 50;
pub const MAX_PARAM_HISTORY: u32 = 20;
pub const MAX_RUBRIC_WEIGHTS: u32 = 6;
pub const MAX_CONDITIONS_PER_MILESTONE: u32 = 5;
pub const MAX_MULTI_TOKEN_TYPES: u32 = 5;
pub const MAX_GROUP_MEMBERS: u32 = 10;
pub const MAX_WAITLIST_SIZE: u32 = 100;
pub const MAX_TIMERS_PER_GRANT: u32 = 10;
pub const MAX_AUCTION_BIDS: u32 = 50;
pub const MAX_BATCH_DETAIL_SIZE: u32 = 10;

// ── Scoring ──────────────────────────────────────────────────────────────────
pub const MAX_SCORE: u32 = 1_000;

// ── Relay ────────────────────────────────────────────────────────────────────
pub const DEFAULT_RELAY_DAILY_LIMIT: u32 = 5;

// ── NFT ──────────────────────────────────────────────────────────────────────
pub const NFT_PROOF_HASH_LEN: u32 = 32;

// ── Analytics ────────────────────────────────────────────────────────────────
pub const ANALYTICS_SNAPSHOT_STALENESS_LEDGERS: u32 = 1_000;

// ── Streaming (#531) ─────────────────────────────────────────────────────────
pub const MAX_STREAM_DURATION_LEDGERS: u32 = 1_000_000;
pub const MIN_STREAM_RATE: i128 = 1;

// ── Quadratic Voting (#537) ──────────────────────────────────────────────────
pub const DEFAULT_VOICE_CREDITS: u32 = 100;

// ── Insurance (#538) ─────────────────────────────────────────────────────────
pub const DEFAULT_INSURANCE_PREMIUM_RATE_BPS: u32 = 50; // 0.5%
pub const DEFAULT_INSURANCE_DURATION_LEDGERS: u32 = 1_000_000;

// ── Hooks (#539) ─────────────────────────────────────────────────────────────
pub const MAX_HOOKS_PER_EVENT: u32 = 5;

// ── DAO Governance (#532) ───────────────────────────────────────────────────
pub const DEFAULT_DAO_VOTING_PERIOD_LEDGERS: u32 = 50_400; // ~7 days
pub const DEFAULT_DAO_QUORUM_VOTES: u64 = 3;
pub const MAX_DAO_TITLE_LEN: u32 = 128;
pub const MAX_DAO_DESCRIPTION_LEN: u32 = 2_048;

// ── Bounty-Mode Grants (#533) ───────────────────────────────────────────────
pub const MAX_BOUNTY_SUBMISSIONS: u32 = 50;

// ── Rate limiting (#544) ─────────────────────────────────────────────────────
pub const RATE_LIMIT_GRANT_CREATE_MAX: u32 = 5;
pub const RATE_LIMIT_GRANT_CREATE_WINDOW: u64 = 3_600;
pub const RATE_LIMIT_MILESTONE_SUBMIT_MAX: u32 = 10;
pub const RATE_LIMIT_MILESTONE_SUBMIT_WINDOW: u64 = 3_600;
pub const RATE_LIMIT_CONTRIBUTOR_REGISTER_MAX: u32 = 3;
pub const RATE_LIMIT_CONTRIBUTOR_REGISTER_WINDOW: u64 = 3_600;
pub const RATE_LIMIT_DISPUTE_RAISE_MAX: u32 = 2;
pub const RATE_LIMIT_DISPUTE_RAISE_WINDOW: u64 = 86_400;
pub const RATE_LIMIT_BOUNTY_CREATE_MAX: u32 = 5;
pub const RATE_LIMIT_BOUNTY_CREATE_WINDOW: u64 = 3_600;

// ── Issue #580: Notification subscriptions ───────────────────────────────────
pub const MAX_SUBSCRIPTIONS_PER_ADDRESS: u32 = 50;

// ── Issue #565: Contributor Portfolio ─────────────────────────────────────────
pub const PORTFOLIO_RECENT_GRANTS_LIMIT: u32 = 5;

// ── Issue #595: Milestone DAG ─────────────────────────────────────────────────
pub const MAX_MILESTONE_DEPS: u32 = 20;

// ── Issue #596: Well-known parameter keys ────────────────────────────────────
pub const PARAM_MAX_GRANT_AMOUNT: &str = "max_grant_amount";
pub const PARAM_MIN_GRANT_AMOUNT: &str = "min_grant_amount";
pub const PARAM_PROTOCOL_FEE_BPS: &str = "protocol_fee_bps";
pub const PARAM_MAX_REVIEWERS: &str = "max_reviewers";
pub const PARAM_QUORUM_THRESHOLD_BPS: &str = "quorum_threshold_bps";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_invariants() {
        assert!(DEFAULT_PROTOCOL_FEE_BPS <= MAX_PROTOCOL_FEE_BPS);
        assert!(MAX_PROTOCOL_FEE_BPS <= BASIS_POINTS_SCALE);
    }

    #[test]
    fn test_page_size_invariants() {
        assert!(DEFAULT_PAGE_SIZE <= MAX_PAGE_SIZE);
        assert!(MAX_PAGE_SIZE > 0);
    }

    #[test]
    fn test_batch_milestone_invariants() {
        assert!(MAX_BATCH_SIZE <= MAX_MILESTONES_PER_GRANT);
        assert!(MAX_BATCH_SIZE > 0);
    }

    #[test]
    fn test_reviewer_count_invariants() {
        assert!(MIN_REVIEWERS_PER_GRANT <= MAX_REVIEWERS_PER_GRANT);
    }

    #[test]
    fn test_ledger_timing_invariants() {
        assert!(DEFAULT_DISPUTE_WINDOW_LEDGERS <= DEFAULT_TIMELOCK_DELAY_LEDGERS);
        assert_eq!(LEDGERS_PER_DAY, DEFAULT_DISPUTE_WINDOW_LEDGERS);
    }

    #[test]
    fn test_amount_invariants() {
        assert!(MIN_GRANT_AMOUNT > 0);
        assert!(MAX_GRANT_AMOUNT > MIN_GRANT_AMOUNT);
    }

    #[test]
    fn test_time_constants() {
        assert_eq!(SECONDS_PER_DAY, 86_400);
        assert_eq!(SECONDS_PER_WEEK, 604_800);
        assert_eq!(SECONDS_PER_YEAR, 31_536_000);
    }

    #[test]
    fn test_lockup_default() {
        assert_eq!(DEFAULT_LOCKUP_DURATION_SECONDS, SECONDS_PER_DAY * 90);
    }

    #[test]
    fn test_sla_defaults() {
        assert_eq!(DEFAULT_REVIEWER_SLA_SECONDS, SECONDS_PER_WEEK);
        assert_eq!(DEFAULT_AUTO_APPROVE_GRACE_SECONDS, SECONDS_PER_DAY * 3);
    }

    #[test]
    fn test_limits_invariants() {
        assert!(MAX_FORK_DEPTH > 0);
        assert!(MAX_INDEX_ENTRIES > 0);
        assert!(MAX_EXPORT_PAGE_SIZE <= MAX_PAGE_SIZE);
    }
}
