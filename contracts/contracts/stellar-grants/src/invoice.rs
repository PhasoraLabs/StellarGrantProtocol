//! # Invoice-Style Milestone Billing
//!
//! Structured milestone invoicing (`submit_invoice`, `approve_invoice`,
//! `reject_invoice`, `resubmit_invoice`, `get_invoice`, and `validate_line_items`).
//!
//! ## Lifecycle & State Machine
//!
//! ```text
//!   [No Invoice] ──submit_invoice (contributor)──> Submitted
//!                                                     │   ▲
//!                       ┌─────────────────────────────┘   │
//!                       │                                 │
//!            approve_invoice (reviewer,           resubmit_invoice
//!             quorum reached & approved)           (contributor)
//!                       │                                 │
//!                       ▼                                 │
//!                    Approved                          Rejected
//!                                                         ▲
//!                                                         │
//!                                            reject_invoice (reviewer)
//! ```
//!
//! ## States (`InvoiceStatus`)
//!
//! * `Submitted` — Initial state on [`submit_invoice`] or after [`resubmit_invoice`].
//!   Awaiting reviewer approval or rejection.
//! * `Approved` — Terminal billing state set by [`approve_invoice`] once
//!   `governance::cast_vote` reaches approval quorum. Milestone payout follows
//!   through the grant milestone release flow.
//! * `Rejected` — Set by [`reject_invoice`] with the rejection reason recorded in
//!   `invoice.notes`. Can be corrected and returned to `Submitted` via
//!   [`resubmit_invoice`].
//! * `Draft` / `Paid` — Defined on [`InvoiceStatus`] in `types.rs` for schema
//!   completeness; not transitioned directly by this module's functions.
//!
//! ## Authorization Model
//!
//! * [`submit_invoice`] and [`resubmit_invoice`] require authorization from the
//!   grant owner (`grant.owner == contributor`).
//! * [`approve_invoice`] and [`reject_invoice`] require authorization from a
//!   registered grant reviewer (`grant.reviewers.contains(reviewer)`).
//! * [`get_invoice`] and [`validate_line_items`] are read-only / pure helpers
//!   and require no authorization.
//!
//! ## Validation & Invariants
//!
//! * Every [`LineItem`] must satisfy `item.total == item.quantity * item.unit_price`.
//! * `tax_bps` is bounded to `0..=10_000` (0%–100%) via `crate::math::basis_points_of`.
//! * Invoice `total` must match `milestone.amount` within a 1% tolerance
//!   (`|total - milestone.amount| <= milestone.amount / 100`).

use crate::errors::ContractError;
use crate::events::Events;
use crate::governance;
use crate::storage::Storage;
use crate::types::{Invoice, InvoiceStatus, LineItem};
use soroban_sdk::{Address, Env, String, Vec};

/// Submit an invoice for a milestone. Contributor only.
///
/// Creates an [`Invoice`] in [`InvoiceStatus::Submitted`] for `(grant_id, milestone_idx)`
/// after verifying line items, tax basis points, and the 1% milestone amount tolerance.
///
/// # Preconditions
/// * Grant `grant_id` and milestone `milestone_idx` must exist.
/// * No invoice may already exist for `(grant_id, milestone_idx)`.
/// * `line_items` must be non-empty, each `total == quantity * unit_price`, and
///   `tax_bps <= 10_000`.
/// * Computed `total` must be within 1% (`milestone.amount / 100`) of `milestone.amount`.
///
/// # Auth
/// * Requires `contributor.require_auth()` and `grant.owner == *contributor`.
///
/// # Arguments
/// * `env` - The Soroban environment.
/// * `contributor` - Grant owner address submitting the invoice.
/// * `grant_id` - Identifier of the target grant.
/// * `milestone_idx` - Zero-based index of the milestone being invoiced.
/// * `invoice_number` - Human-readable invoice reference string.
/// * `line_items` - Non-empty itemized charges making up the invoice.
/// * `tax_bps` - Tax rate in basis points (`0..=10_000`).
/// * `notes` - Optional contributor notes attached to the invoice.
///
/// # Returns
/// * `Ok(())` when the invoice is stored and `InvoiceSubmitted` is emitted.
///
/// # Errors
/// * [`ContractError::GrantNotFound`] - If `grant_id` does not exist.
/// * [`ContractError::Unauthorized`] - If `contributor` is not `grant.owner`.
/// * [`ContractError::MilestoneNotFound`] - If `(grant_id, milestone_idx)` does not exist.
/// * [`ContractError::InvoiceAlreadySubmitted`] - If an invoice already exists for this milestone.
/// * [`ContractError::InvalidInput`] - If `line_items` is empty, any line item total mismatches
///   `quantity * unit_price`, `tax_bps > 10_000`, or `total` deviates from `milestone.amount`
///   by more than 1%.
///
/// # Events
/// * Emits `InvoiceSubmitted` via `Events::invoice_submitted`.
pub fn submit_invoice(
    env: &Env,
    contributor: &Address,
    grant_id: u64,
    milestone_idx: u32,
    invoice_number: String,
    line_items: Vec<LineItem>,
    tax_bps: u32,
    notes: Option<String>,
) -> Result<(), ContractError> {
    contributor.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *contributor {
        return Err(ContractError::Unauthorized);
    }

    let milestone = Storage::get_milestone(env, grant_id, milestone_idx)
        .ok_or(ContractError::MilestoneNotFound)?;

    // Check if invoice already exists
    if Storage::get_invoice(env, grant_id, milestone_idx).is_some() {
        return Err(ContractError::InvoiceAlreadySubmitted);
    }

    // Validate line items and calculate totals
    let (subtotal, total) = validate_line_items(&line_items, tax_bps)?;

    // Verify total matches milestone amount (within small tolerance)
    let tolerance = milestone.amount / 100; // 1% tolerance
    let diff = if total > milestone.amount {
        total - milestone.amount
    } else {
        milestone.amount - total
    };

    if diff > tolerance {
        return Err(ContractError::InvalidInput);
    }

    let invoice = Invoice {
        grant_id,
        milestone_idx,
        invoice_number: invoice_number.clone(),
        contributor: contributor.clone(),
        line_items,
        subtotal,
        tax_bps,
        total,
        currency_token: grant.token.clone(),
        status: InvoiceStatus::Submitted,
        submitted_at: env.ledger().timestamp(),
        approved_at: None,
        notes,
    };

    Storage::set_invoice(env, grant_id, milestone_idx, &invoice);

    Events::invoice_submitted(env, grant_id, milestone_idx, invoice_number, total);

    Ok(())
}

/// Approve an invoice. Reviewer only. Triggers milestone approval.
///
/// Casts an approval vote for `(grant_id, milestone_idx)` via `governance::cast_vote`.
/// If the vote reaches quorum and approves the milestone, transitions the invoice from
/// [`InvoiceStatus::Submitted`] to [`InvoiceStatus::Approved`], records `approved_at`,
/// persists the updated invoice and milestone, and emits `InvoiceApproved`.
/// If quorum is not yet reached, the vote is recorded while the invoice remains `Submitted`.
///
/// # Preconditions
/// * Grant `grant_id`, milestone `milestone_idx`, and an invoice in
///   [`InvoiceStatus::Submitted`] must exist.
/// * Milestone must be in a votable state required by `governance::cast_vote`.
///
/// # Auth
/// * Requires `reviewer.require_auth()` and `grant.reviewers.contains(reviewer)`.
///
/// # Arguments
/// * `env` - The Soroban environment.
/// * `reviewer` - Address of the grant reviewer approving the invoice.
/// * `grant_id` - Identifier of the target grant.
/// * `milestone_idx` - Zero-based index of the invoiced milestone.
///
/// # Returns
/// * `Ok(())` once the approval vote is recorded (and invoice finalized if quorum is met).
///
/// # Errors
/// * [`ContractError::Unauthorized`] - If `reviewer` is not in `grant.reviewers`.
/// * [`ContractError::InvoiceNotFound`] - If no invoice exists for `(grant_id, milestone_idx)`.
/// * [`ContractError::InvalidState`] - If the invoice status is not [`InvoiceStatus::Submitted`].
/// * Propagates voting errors from `governance::cast_vote` (e.g.
///   [`ContractError::MilestoneNotSubmitted`], [`ContractError::AlreadyVoted`],
///   [`ContractError::InvalidState`]).
///
/// # Events
/// * Emits `InvoiceApproved` via `Events::invoice_approved` when quorum approves the milestone.
pub fn approve_invoice(
    env: &Env,
    reviewer: &Address,
    grant_id: u64,
    milestone_idx: u32,
) -> Result<(), ContractError> {
    reviewer.require_auth();

    let mut grant = Storage::get_grant_v(env, grant_id);
    let mut milestone = Storage::get_milestone_v(env, grant_id, milestone_idx);

    if !grant.reviewers.contains(reviewer.clone()) {
        return Err(ContractError::Unauthorized);
    }

    let mut invoice =
        Storage::get_invoice(env, grant_id, milestone_idx).ok_or(ContractError::InvoiceNotFound)?;

    if invoice.status != InvoiceStatus::Submitted {
        return Err(ContractError::InvalidState);
    }

    // Cast vote to approve milestone
    let result = governance::cast_vote(env, &mut grant, &mut milestone, reviewer, true, None)?;

    if result.quorum_reached && result.approved {
        invoice.status = InvoiceStatus::Approved;
        invoice.approved_at = Some(env.ledger().timestamp());

        Storage::set_invoice(env, grant_id, milestone_idx, &invoice);
        Storage::set_milestone(env, grant_id, milestone_idx, &milestone);

        Events::invoice_approved(env, grant_id, milestone_idx, reviewer.clone());
    }

    Ok(())
}

/// Reject an invoice with a reason. Reviewer only.
///
/// Transitions an invoice from [`InvoiceStatus::Submitted`] to [`InvoiceStatus::Rejected`]
/// and overwrites `invoice.notes` with `reason` so the contributor can inspect why it was
/// denied and submit corrections via [`resubmit_invoice`].
///
/// # Preconditions
/// * Grant `grant_id` and an invoice for `(grant_id, milestone_idx)` must exist.
/// * Invoice must currently be in [`InvoiceStatus::Submitted`].
///
/// # Auth
/// * Requires `reviewer.require_auth()` and `grant.reviewers.contains(reviewer)`.
///
/// # Arguments
/// * `env` - The Soroban environment.
/// * `reviewer` - Address of the grant reviewer rejecting the invoice.
/// * `grant_id` - Identifier of the target grant.
/// * `milestone_idx` - Zero-based index of the invoiced milestone.
/// * `reason` - Explanation stored in `invoice.notes` and emitted in the rejection event.
///
/// # Returns
/// * `Ok(())` when the invoice is marked [`InvoiceStatus::Rejected`].
///
/// # Errors
/// * [`ContractError::Unauthorized`] - If `reviewer` is not in `grant.reviewers`.
/// * [`ContractError::InvoiceNotFound`] - If no invoice exists for `(grant_id, milestone_idx)`.
/// * [`ContractError::InvalidState`] - If the invoice status is not [`InvoiceStatus::Submitted`].
///
/// # Events
/// * Emits `InvoiceRejected` via `Events::invoice_rejected`.
pub fn reject_invoice(
    env: &Env,
    reviewer: &Address,
    grant_id: u64,
    milestone_idx: u32,
    reason: String,
) -> Result<(), ContractError> {
    reviewer.require_auth();

    let grant = Storage::get_grant_v(env, grant_id);

    if !grant.reviewers.contains(reviewer.clone()) {
        return Err(ContractError::Unauthorized);
    }

    let mut invoice =
        Storage::get_invoice(env, grant_id, milestone_idx).ok_or(ContractError::InvoiceNotFound)?;

    if invoice.status != InvoiceStatus::Submitted {
        return Err(ContractError::InvalidState);
    }

    invoice.status = InvoiceStatus::Rejected;
    invoice.notes = Some(reason.clone());

    Storage::set_invoice(env, grant_id, milestone_idx, &invoice);

    Events::invoice_rejected(env, grant_id, milestone_idx, reviewer.clone(), reason);

    Ok(())
}

/// Resubmit a rejected invoice with corrections. Contributor only.
///
/// Replaces the invoice's line items, recalculates `subtotal` and `total` using the
/// existing `invoice.tax_bps`, validates the 1% milestone tolerance, clears `notes`,
/// updates `submitted_at`, and transitions status from [`InvoiceStatus::Rejected`] back
/// to [`InvoiceStatus::Submitted`].
///
/// # Preconditions
/// * Grant `grant_id`, milestone `milestone_idx`, and an existing invoice must exist.
/// * Invoice must currently be in [`InvoiceStatus::Rejected`].
/// * `updated_items` must be non-empty and satisfy [`validate_line_items`] and the
///   1% `milestone.amount` tolerance.
///
/// # Auth
/// * Requires `contributor.require_auth()` and `grant.owner == *contributor`.
///
/// # Arguments
/// * `env` - The Soroban environment.
/// * `contributor` - Grant owner address resubmitting the invoice.
/// * `grant_id` - Identifier of the target grant.
/// * `milestone_idx` - Zero-based index of the invoiced milestone.
/// * `updated_items` - Revised list of [`LineItem`] entries.
///
/// # Returns
/// * `Ok(())` when the invoice returns to [`InvoiceStatus::Submitted`].
///
/// # Errors
/// * [`ContractError::GrantNotFound`] - If `grant_id` does not exist.
/// * [`ContractError::Unauthorized`] - If `contributor` is not `grant.owner`.
/// * [`ContractError::MilestoneNotFound`] - If `(grant_id, milestone_idx)` does not exist.
/// * [`ContractError::InvoiceNotFound`] - If no invoice exists for `(grant_id, milestone_idx)`.
/// * [`ContractError::InvalidState`] - If the invoice status is not [`InvoiceStatus::Rejected`].
/// * [`ContractError::InvalidInput`] - If `updated_items` is invalid or `total` deviates from
///   `milestone.amount` by more than 1%.
///
/// # Events
/// * Emits `InvoiceResubmitted` via `Events::invoice_resubmitted`.
pub fn resubmit_invoice(
    env: &Env,
    contributor: &Address,
    grant_id: u64,
    milestone_idx: u32,
    updated_items: Vec<LineItem>,
) -> Result<(), ContractError> {
    contributor.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *contributor {
        return Err(ContractError::Unauthorized);
    }

    let milestone = Storage::get_milestone(env, grant_id, milestone_idx)
        .ok_or(ContractError::MilestoneNotFound)?;

    let mut invoice =
        Storage::get_invoice(env, grant_id, milestone_idx).ok_or(ContractError::InvoiceNotFound)?;

    if invoice.status != InvoiceStatus::Rejected {
        return Err(ContractError::InvalidState);
    }

    // Validate new line items
    let (subtotal, total) = validate_line_items(&updated_items, invoice.tax_bps)?;

    // Verify total matches milestone amount
    let tolerance = milestone.amount / 100;
    let diff = if total > milestone.amount {
        total - milestone.amount
    } else {
        milestone.amount - total
    };

    if diff > tolerance {
        return Err(ContractError::InvalidInput);
    }

    invoice.line_items = updated_items;
    invoice.subtotal = subtotal;
    invoice.total = total;
    invoice.status = InvoiceStatus::Submitted;
    invoice.submitted_at = env.ledger().timestamp();
    invoice.notes = None;

    Storage::set_invoice(env, grant_id, milestone_idx, &invoice);

    Events::invoice_resubmitted(env, grant_id, milestone_idx, total);

    Ok(())
}

/// Return the invoice for a milestone, if one has been submitted.
///
/// # Auth
/// * Read-only helper; requires no authorization.
///
/// # Arguments
/// * `env` - The Soroban environment.
/// * `grant_id` - Identifier of the grant.
/// * `milestone_idx` - Zero-based index of the milestone.
///
/// # Returns
/// * `Some(Invoice)` if an invoice exists for `(grant_id, milestone_idx)`, otherwise `None`.
pub fn get_invoice(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<Invoice> {
    Storage::get_invoice(env, grant_id, milestone_idx)
}

/// Validate line items and compute `(subtotal, total)` including tax.
///
/// Ensures `items` is non-empty, verifies each line item satisfies
/// `item.total == item.quantity * item.unit_price`, sums all line totals into `subtotal`,
/// and computes tax using `crate::math::basis_points_of(subtotal, tax_bps)`.
///
/// # Auth
/// * Pure validation helper; requires no authorization.
///
/// # Arguments
/// * `items` - List of invoice [`LineItem`] entries.
/// * `tax_bps` - Tax rate in basis points (`0..=10_000`).
///
/// # Returns
/// * `Ok((subtotal, total))` where `total = subtotal + tax_amount`.
///
/// # Errors
/// * [`ContractError::InvalidInput`] - If `items` is empty, any `item.total` does not equal
///   `(item.quantity as i128) * item.unit_price`, or `tax_bps > 10_000` / overflows.
pub fn validate_line_items(
    items: &Vec<LineItem>,
    tax_bps: u32,
) -> Result<(i128, i128), ContractError> {
    if items.is_empty() {
        return Err(ContractError::InvalidInput);
    }

    let mut subtotal: i128 = 0;

    for item in items.iter() {
        // Validate each line item total
        let expected_total = (item.quantity as i128) * item.unit_price;
        if item.total != expected_total {
            return Err(ContractError::InvalidInput);
        }
        subtotal = subtotal.saturating_add(item.total);
    }

    // Calculate tax. `basis_points_of` rejects tax_bps > 10_000 and uses
    // checked arithmetic internally, so a malicious/oversized tax_bps
    // returns a clean error instead of overflowing.
    let tax_amount = crate::math::basis_points_of(subtotal, tax_bps)?;
    let total = subtotal.saturating_add(tax_amount);

    Ok((subtotal, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::types::{Grant, GrantStatus, Milestone, MilestoneState};
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, Map};

    fn setup_grant(env: &Env, owner: Address, token: Address) -> Grant {
        Grant {
            id: 1,
            owner: owner.clone(),
            title: String::from_str(env, "Test Grant"),
            description: String::from_str(env, "Test"),
            token: token.clone(),
            status: GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 500,
            reviewers: Vec::new(env),
            total_milestones: 1,
            milestones_paid_out: 0,
            escrow_balance: 500,
            funders: Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        }
    }

    fn setup_milestone(env: &Env, amount: i128) -> Milestone {
        Milestone {
            idx: 0,
            description: String::from_str(env, "Milestone"),
            amount,
            state: MilestoneState::Submitted,
            votes: Map::new(env),
            approvals: 0,
            rejections: 0,
            reasons: Map::new(env),
            status_updated_at: env.ledger().timestamp(),
            proof_url: None,
            submission_timestamp: env.ledger().timestamp(),
            deadline: None,
            reviewer_count_snapshot: 0,
            reviewer_list_snapshot: soroban_sdk::Vec::new(env),
        }
    }

    fn line_item(env: &Env, quantity: u32, unit_price: i128) -> LineItem {
        LineItem {
            description: String::from_str(env, "Work"),
            quantity,
            unit_price,
            total: (quantity as i128) * unit_price,
        }
    }

    #[test]
    fn test_validate_line_items_accepts_valid_tax_bps() {
        let env = Env::default();
        let items = Vec::from_array(&env, [line_item(&env, 10, 100)]);

        // 1000 subtotal, 2.5% tax = 25
        let (subtotal, total) = validate_line_items(&items, 250).unwrap();
        assert_eq!(subtotal, 1000);
        assert_eq!(total, 1025);
    }

    #[test]
    fn test_validate_line_items_rejects_excessive_tax_bps() {
        let env = Env::default();
        let items = Vec::from_array(&env, [line_item(&env, 10, 100)]);

        // A malicious/oversized tax_bps must return a clean error, not
        // silently overflow or panic.
        let result = validate_line_items(&items, u32::MAX);
        assert_eq!(result, Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_validate_line_items_rejects_tax_bps_just_over_limit() {
        let env = Env::default();
        let items = Vec::from_array(&env, [line_item(&env, 10, 100)]);

        let result = validate_line_items(&items, 10_001);
        assert_eq!(result, Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_submit_invoice_rejects_tax_bps_over_10000() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::StellarGrantsContract, ());

        env.as_contract(&contract_id, || {
            let owner = Address::generate(&env);
            let token = Address::generate(&env);

            let grant = setup_grant(&env, owner.clone(), token);
            Storage::set_grant(&env, 1, &grant);

            let milestone = setup_milestone(&env, 1000);
            Storage::set_milestone(&env, 1, 0, &milestone);

            // Subtotal 1000 so the milestone-amount tolerance check never
            // masks the tax_bps rejection.
            let items = Vec::from_array(&env, [line_item(&env, 10, 100)]);
            let invoice_number = String::from_str(&env, "INV-1");

            let result = submit_invoice(&env, &owner, 1, 0, invoice_number, items, 10_001, None);
            assert_eq!(result, Err(ContractError::InvalidInput));

            // A malicious submission must not leave a partial invoice on record.
            assert!(get_invoice(&env, 1, 0).is_none());
        });
    }
}
