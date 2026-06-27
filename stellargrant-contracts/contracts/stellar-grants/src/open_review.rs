/// Public crowdsourced review module (issue #590).
/// Any registered contributor may submit a non-binding signal (thumbs up/down with comment)
/// on a milestone. Reviews are visible to formal reviewers but do not affect governance votes.
use soroban_sdk::{Address, Env, String, Vec};

use crate::constants::MAX_PUBLIC_REVIEW_COMMENT_LEN;
use crate::errors::ContractError;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{PublicReview, PublicReviewSignal};

/// Submit or update a public review for a milestone.
/// One review per (reviewer, grant_id, milestone_idx). Resubmission updates the existing entry
/// and preserves the accumulated `helpful_votes`.
pub fn submit_review(
    env: &Env,
    reviewer: &Address,
    grant_id: u64,
    milestone_idx: u32,
    signal: PublicReviewSignal,
    comment: String,
) -> Result<(), ContractError> {
    reviewer.require_auth();

    if comment.len() > MAX_PUBLIC_REVIEW_COMMENT_LEN {
        return Err(ContractError::CommentTooLong);
    }

    let profile = Storage::get_contributor(env, reviewer.clone());
    let reviewer_reputation = profile.map(|p| p.reputation_score as u32).unwrap_or(0);

    let prior_helpful = Storage::get_public_reviewer_record(env, reviewer, grant_id, milestone_idx)
        .map(|r| r.helpful_votes)
        .unwrap_or(0);

    let review = PublicReview {
        reviewer: reviewer.clone(),
        grant_id,
        milestone_idx,
        signal,
        comment,
        reviewer_reputation,
        submitted_at: env.ledger().timestamp(),
        helpful_votes: prior_helpful,
    };

    let mut reviews = Storage::get_public_reviews(env, grant_id, milestone_idx);
    let mut found_idx: Option<u32> = None;
    for i in 0..reviews.len() {
        let r = reviews.get(i).unwrap();
        if r.reviewer == *reviewer {
            found_idx = Some(i);
            break;
        }
    }
    if let Some(idx) = found_idx {
        reviews.set(idx, review.clone());
    } else {
        reviews.push_back(review.clone());
    }

    Storage::set_public_reviews(env, grant_id, milestone_idx, &reviews);
    Storage::set_public_reviewer_record(env, reviewer, grant_id, milestone_idx, &review);

    Events::emit_public_review_submitted(env, grant_id, milestone_idx, reviewer.clone());
    Ok(())
}

/// Vote a public review as helpful. Any address may cast a helpful vote; duplicates are allowed
/// (the contract does not enforce unique voters per review to stay gas-light).
pub fn mark_helpful(
    env: &Env,
    voter: &Address,
    grant_id: u64,
    milestone_idx: u32,
    reviewer: &Address,
) -> Result<(), ContractError> {
    voter.require_auth();

    let mut review = Storage::get_public_reviewer_record(env, reviewer, grant_id, milestone_idx)
        .ok_or(ContractError::ReviewNotFound)?;

    review.helpful_votes = review.helpful_votes.saturating_add(1);
    Storage::set_public_reviewer_record(env, reviewer, grant_id, milestone_idx, &review);

    let mut reviews = Storage::get_public_reviews(env, grant_id, milestone_idx);
    for i in 0..reviews.len() {
        let mut r = reviews.get(i).unwrap();
        if r.reviewer == *reviewer {
            r.helpful_votes = review.helpful_votes;
            reviews.set(i, r);
            break;
        }
    }
    Storage::set_public_reviews(env, grant_id, milestone_idx, &reviews);

    Events::emit_review_marked_helpful(
        env,
        grant_id,
        milestone_idx,
        reviewer.clone(),
        voter.clone(),
    );
    Ok(())
}

/// Return all public reviews for a milestone, sorted by `reviewer_reputation` descending.
pub fn get_reviews(env: &Env, grant_id: u64, milestone_idx: u32) -> Vec<PublicReview> {
    let reviews = Storage::get_public_reviews(env, grant_id, milestone_idx);
    let len = reviews.len();
    if len <= 1 {
        return reviews;
    }
    // Bubble sort (small n ≤ MAX_PAGE_SIZE, no recursion)
    let mut sorted = reviews;
    for i in 0..len {
        for j in 0..(len.saturating_sub(1 + i)) {
            let a = sorted.get(j).unwrap();
            let b = sorted.get(j + 1).unwrap();
            if a.reviewer_reputation < b.reviewer_reputation {
                sorted.set(j, b);
                sorted.set(j + 1, a);
            }
        }
    }
    sorted
}

/// Return aggregated signal counts: (positive, neutral, negative).
pub fn aggregate_signals(env: &Env, grant_id: u64, milestone_idx: u32) -> (u32, u32, u32) {
    let reviews = Storage::get_public_reviews(env, grant_id, milestone_idx);
    let mut positive = 0u32;
    let mut neutral = 0u32;
    let mut negative = 0u32;
    for r in reviews.iter() {
        match r.signal {
            PublicReviewSignal::Positive => positive = positive.saturating_add(1),
            PublicReviewSignal::Neutral => neutral = neutral.saturating_add(1),
            PublicReviewSignal::Negative => negative = negative.saturating_add(1),
        }
    }
    (positive, neutral, negative)
}

/// Return a specific reviewer's public review for a milestone, if any.
pub fn get_review(
    env: &Env,
    reviewer: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Option<PublicReview> {
    Storage::get_public_reviewer_record(env, reviewer, grant_id, milestone_idx)
}

/// Return true if the given address has already submitted a public review for this milestone.
pub fn has_reviewed(env: &Env, reviewer: &Address, grant_id: u64, milestone_idx: u32) -> bool {
    Storage::get_public_reviewer_record(env, reviewer, grant_id, milestone_idx).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    #[test]
    fn submit_updates_aggregate_signals() {
        let env = Env::default();
        env.mock_all_auths();

        let reviewer = Address::generate(&env);
        let grant_id = 1u64;
        let milestone_idx = 0u32;

        submit_review(
            &env,
            &reviewer,
            grant_id,
            milestone_idx,
            PublicReviewSignal::Positive,
            String::from_str(&env, "Great work!"),
        )
        .unwrap();

        let (pos, neu, neg) = aggregate_signals(&env, grant_id, milestone_idx);
        assert_eq!(pos, 1);
        assert_eq!(neu, 0);
        assert_eq!(neg, 0);
    }

    #[test]
    fn mark_helpful_increments_count() {
        let env = Env::default();
        env.mock_all_auths();

        let reviewer = Address::generate(&env);
        let voter = Address::generate(&env);

        submit_review(
            &env,
            &reviewer,
            1,
            0,
            PublicReviewSignal::Neutral,
            String::from_str(&env, "OK"),
        )
        .unwrap();

        mark_helpful(&env, &voter, 1, 0, &reviewer).unwrap();

        let review = get_review(&env, &reviewer, 1, 0).unwrap();
        assert_eq!(review.helpful_votes, 1);
    }

    #[test]
    fn has_reviewed_returns_true_after_submission() {
        let env = Env::default();
        env.mock_all_auths();

        let reviewer = Address::generate(&env);
        assert!(!has_reviewed(&env, &reviewer, 1, 0));

        submit_review(
            &env,
            &reviewer,
            1,
            0,
            PublicReviewSignal::Negative,
            String::from_str(&env, "Needs work"),
        )
        .unwrap();

        assert!(has_reviewed(&env, &reviewer, 1, 0));
    }

    #[test]
    fn three_reviews_aggregate_reflects_counts() {
        let env = Env::default();
        env.mock_all_auths();

        for signal in [
            PublicReviewSignal::Positive,
            PublicReviewSignal::Positive,
            PublicReviewSignal::Negative,
        ] {
            let reviewer = Address::generate(&env);
            submit_review(
                &env,
                &reviewer,
                2,
                1,
                signal,
                String::from_str(&env, "comment"),
            )
            .unwrap();
        }

        let (pos, neu, neg) = aggregate_signals(&env, 2, 1);
        assert_eq!(pos, 2);
        assert_eq!(neu, 0);
        assert_eq!(neg, 1);
    }

    #[test]
    fn comment_exceeding_max_length_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let reviewer = Address::generate(&env);
        // Build a 501-char string
        let mut s = soroban_sdk::String::from_str(&env, "");
        for _ in 0..501 {
            s = soroban_sdk::String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
            break; // 501 chars
        }

        let result = submit_review(&env, &reviewer, 1, 0, PublicReviewSignal::Positive, s);
        assert_eq!(result, Err(ContractError::CommentTooLong));
    }
}
