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

    // Calculate tax
    let tax_amount = (subtotal * (tax_bps as i128)) / 10_000;
    let total = subtotal.saturating_add(tax_amount);

    Ok((subtotal, total))
}
