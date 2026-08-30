use crate::errors::ContractError;
use crate::events::Events;
use crate::governance;
use crate::storage::Storage;
use crate::types::{Invoice, InvoiceStatus, LineItem};
use soroban_sdk::{Address, Env, String, Vec};

/// Submit an invoice for a milestone. Contributor only.
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

/// Resubmit a rejected invoice with corrections.
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

/// Return the invoice for a milestone.
pub fn get_invoice(env: &Env, grant_id: u64, milestone_idx: u32) -> Option<Invoice> {
    Storage::get_invoice(env, grant_id, milestone_idx)
}

/// Validate line items: each `total == quantity * unit_price`, grand total matches sum of line items + tax.
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
