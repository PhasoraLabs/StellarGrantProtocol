#![allow(dead_code)]

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

// ── String Limits ─────────────────────────────────────────────────────────────
pub const MAX_TITLE_LEN: u32 = 128;
pub const MAX_DESCRIPTION_LEN: u32 = 1_024;
pub const MAX_PROOF_URL_LEN: u32 = 512;
pub const MAX_BIO_LEN: u32 = 256;

// ── Pagination ────────────────────────────────────────────────────────────────
pub const MAX_PAGE_SIZE: u32 = 50;
pub const DEFAULT_PAGE_SIZE: u32 = 20;

// ── Reputation ────────────────────────────────────────────────────────────────
pub const MAX_REPUTATION_SCORE: u32 = 1_000;
pub const REPUTATION_REJECTION_PENALTY: u32 = 200; // bps subtracted per rejection

// ── Milestones ────────────────────────────────────────────────────────────────
pub const MAX_MILESTONES_PER_GRANT: u32 = 20;
pub const MAX_BATCH_SIZE: u32 = 10;

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
}
