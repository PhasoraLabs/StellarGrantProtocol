use soroban_sdk::{contractevent, token, Address, Env};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{GrantStatus, PaymentStream, StreamStatus};

// ── Events ────────────────────────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamCreated {
    pub stream_id: u32,
    pub grant_id: u64,
    pub sender: Address,
    pub recipient: Address,
    pub rate_per_ledger: i128,
    pub deposited: i128,
    pub end_ledger: u32,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamWithdrawn {
    pub stream_id: u32,
    pub recipient: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamCancelled {
    pub stream_id: u32,
    pub sender_refund: i128,
    pub recipient_payout: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamPaused {
    pub stream_id: u32,
    pub paused_at_ledger: u32,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamResumed {
    pub stream_id: u32,
    pub new_end_ledger: u32,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Create a new payment stream. Sender deposits `rate_per_ledger * duration_ledgers` upfront.
pub fn create_stream(
    env: &Env,
    sender: &Address,
    recipient: &Address,
    grant_id: u64,
    token: &Address,
    rate_per_ledger: i128,
    duration_ledgers: u32,
) -> Result<u32, ContractError> {
    crate::reentrancy::protect(env)?;
    sender.require_auth();

    if rate_per_ledger <= 0 {
        return Err(ContractError::InvalidInput);
    }
    if duration_ledgers == 0 {
        return Err(ContractError::InvalidInput);
    }

    // Issue #814: streams must be tied to a real, active grant.
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.status != GrantStatus::Active {
        return Err(ContractError::InvalidState);
    }

    let deposited = rate_per_ledger
        .checked_mul(duration_ledgers as i128)
        .ok_or(ContractError::InvalidInput)?;

    let start_ledger = env.ledger().sequence();
    let end_ledger = start_ledger
        .checked_add(duration_ledgers)
        .ok_or(ContractError::InvalidInput)?;

    let token_client = token::Client::new(env, token);
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(sender, env.current_contract_address(), &deposited);
        Ok(())
    })?;

    let stream_id = Storage::next_stream_id(env);
    let stream = PaymentStream {
        id: stream_id,
        grant_id,
        sender: sender.clone(),
        recipient: recipient.clone(),
        token: token.clone(),
        rate_per_ledger,
        deposited,
        withdrawn: 0,
        start_ledger,
        end_ledger,
        status: StreamStatus::Active,
        created_at: env.ledger().timestamp(),
        paused_at_ledger: 0,
    };

    Storage::set_stream(env, &stream);

    StreamCreated {
        stream_id,
        grant_id,
        sender: sender.clone(),
        recipient: recipient.clone(),
        rate_per_ledger,
        deposited,
        end_ledger,
    }
    .publish(env);

    Ok(stream_id)
}

/// Recipient withdraws all accrued-but-unclaimed tokens.
pub fn withdraw_stream(
    env: &Env,
    recipient: &Address,
    stream_id: u32,
) -> Result<i128, ContractError> {
    crate::reentrancy::protect(env)?;
    recipient.require_auth();

    let mut stream = Storage::get_stream(env, stream_id).ok_or(ContractError::StreamNotFound)?;

    if stream.recipient != *recipient {
        return Err(ContractError::Unauthorized);
    }
    if stream.status != StreamStatus::Active && stream.status != StreamStatus::Paused {
        return Err(ContractError::StreamNotActive);
    }

    let claimable = accrued_amount(env, &stream);
    if claimable == 0 {
        return Ok(0);
    }

    stream.withdrawn = stream
        .withdrawn
        .checked_add(claimable)
        .ok_or(ContractError::InvalidInput)?;

    // Mark completed if fully drained
    if stream.withdrawn >= stream.deposited {
        stream.status = StreamStatus::Completed;
    }

    Storage::set_stream(env, &stream);

    let token_client = token::Client::new(env, &stream.token);
    crate::reentrancy::protect_external_call(env, || {
        token_client.transfer(&env.current_contract_address(), recipient, &claimable);
        Ok(())
    })?;

    StreamWithdrawn {
        stream_id,
        recipient: recipient.clone(),
        amount: claimable,
    }
    .publish(env);

    Ok(claimable)
}

/// Compute how many tokens have accrued since stream start up to current ledger.
pub fn accrued_amount(env: &Env, stream: &PaymentStream) -> i128 {
    let current = match stream.status {
        StreamStatus::Active => env.ledger().sequence(),
        StreamStatus::Paused => stream.paused_at_ledger,
        _ => return 0,
    };

    let elapsed = if current >= stream.end_ledger {
        (stream.end_ledger - stream.start_ledger) as i128
    } else {
        (current - stream.start_ledger) as i128
    };
    let total_accrued = elapsed
        .saturating_mul(stream.rate_per_ledger)
        .min(stream.deposited);
    total_accrued.saturating_sub(stream.withdrawn).max(0)
}

/// Cancel a stream. Sender gets back unstreamed portion; recipient gets accrued.
pub fn cancel_stream(
    env: &Env,
    sender: &Address,
    stream_id: u32,
) -> Result<(i128, i128), ContractError> {
    crate::reentrancy::protect(env)?;
    sender.require_auth();

    let mut stream = Storage::get_stream(env, stream_id).ok_or(ContractError::StreamNotFound)?;

    if stream.sender != *sender {
        return Err(ContractError::Unauthorized);
    }
    if stream.status != StreamStatus::Active {
        return Err(ContractError::StreamNotActive);
    }

    let recipient_payout = accrued_amount(env, &stream);
    let sender_refund = stream
        .deposited
        .saturating_sub(stream.withdrawn)
        .saturating_sub(recipient_payout);

    stream.status = StreamStatus::Cancelled;
    stream.withdrawn = stream.deposited; // mark fully consumed
    Storage::set_stream(env, &stream);

    let token_client = token::Client::new(env, &stream.token);

    crate::reentrancy::protect_external_call(env, || {
        if recipient_payout > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &stream.recipient,
                &recipient_payout,
            );
        }
        if sender_refund > 0 {
            token_client.transfer(&env.current_contract_address(), sender, &sender_refund);
        }
        Ok(())
    })?;

    StreamCancelled {
        stream_id,
        sender_refund,
        recipient_payout,
    }
    .publish(env);

    Ok((sender_refund, recipient_payout))
}

/// Pause a stream (sender only). Accrual stops at current ledger.
pub fn pause_stream(env: &Env, sender: &Address, stream_id: u32) -> Result<(), ContractError> {
    sender.require_auth();

    let mut stream = Storage::get_stream(env, stream_id).ok_or(ContractError::StreamNotFound)?;

    if stream.sender != *sender {
        return Err(ContractError::Unauthorized);
    }
    if stream.status != StreamStatus::Active {
        return Err(ContractError::StreamNotActive);
    }

    let paused_at = env.ledger().sequence();
    stream.status = StreamStatus::Paused;
    stream.paused_at_ledger = paused_at;
    Storage::set_stream(env, &stream);

    StreamPaused {
        stream_id,
        paused_at_ledger: paused_at,
    }
    .publish(env);

    Ok(())
}

/// Resume a paused stream. Adjusts end_ledger by the pause duration.
pub fn resume_stream(env: &Env, sender: &Address, stream_id: u32) -> Result<(), ContractError> {
    sender.require_auth();

    let mut stream = Storage::get_stream(env, stream_id).ok_or(ContractError::StreamNotFound)?;

    if stream.sender != *sender {
        return Err(ContractError::Unauthorized);
    }
    if stream.status != StreamStatus::Paused {
        return Err(ContractError::StreamNotActive);
    }

    let current = env.ledger().sequence();
    let pause_duration = current.saturating_sub(stream.paused_at_ledger);
    stream.end_ledger = stream
        .end_ledger
        .checked_add(pause_duration)
        .ok_or(ContractError::InvalidInput)?;
    stream.status = StreamStatus::Active;
    stream.paused_at_ledger = 0;
    Storage::set_stream(env, &stream);

    StreamResumed {
        stream_id,
        new_end_ledger: stream.end_ledger,
    }
    .publish(env);

    Ok(())
}

/// Return stream details by id.
pub fn get_stream(env: &Env, stream_id: u32) -> Result<PaymentStream, ContractError> {
    Storage::get_stream(env, stream_id).ok_or(ContractError::StreamNotFound)
}

// ── Unit Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Grant;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{token::StellarAssetClient, Address, Env, String};

    fn setup() -> (
        Env,
        Address,
        Address,
        Address,
        Address,
        soroban_sdk::Address,
    ) {
        let env = Env::default();
        let contract_id = env.register(crate::StellarGrantsContract, ());
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        let stellar_asset = StellarAssetClient::new(&env, &token_contract);
        stellar_asset.mint(&sender, &10_000_000);
        let _ = admin;

        // Streams must reference a real, active grant (Issue #814).
        env.as_contract(&contract_id, || {
            create_active_grant(&env, 1, &sender, &token_contract, GrantStatus::Active);
        });

        (
            env,
            sender,
            recipient,
            token_contract,
            token_admin,
            contract_id,
        )
    }

    fn create_active_grant(
        env: &Env,
        id: u64,
        owner: &Address,
        token: &Address,
        status: GrantStatus,
    ) {
        let grant = Grant {
            id,
            owner: owner.clone(),
            title: String::from_str(env, "test grant"),
            description: String::from_str(env, "desc"),
            token: token.clone(),
            status,
            total_amount: 1_000_000,
            milestone_amount: 100_000,
            reviewers: soroban_sdk::Vec::new(env),
            total_milestones: 1,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: soroban_sdk::Vec::new(env),
            reason: None,
            timestamp: env.ledger().timestamp(),
            require_compliance: None,
        };
        Storage::set_grant(env, id, &grant);
    }

    #[test]
    fn test_accrual_at_midpoint_returns_50_percent() {
        let (env, sender, recipient, token, _, cid) = setup();
        let client = crate::StellarGrantsContractClient::new(&env, &cid);
        let stream_id = client.create_stream(&sender, &recipient, &1, &token, &100, &100);
        // Advance to midpoint
        env.ledger().with_mut(|li| li.sequence_number += 50);
        let stream = client.get_stream(&stream_id);
        let accrued = accrued_amount(&env, &stream);
        assert_eq!(accrued, 5_000); // 50 ledgers * 100 rate
    }

    #[test]
    fn test_double_withdraw_returns_zero_second_time() {
        let (env, sender, recipient, token, _, cid) = setup();
        let client = crate::StellarGrantsContractClient::new(&env, &cid);
        let stream_id = client.create_stream(&sender, &recipient, &1, &token, &100, &100);
        env.ledger().with_mut(|li| li.sequence_number += 50);
        let first = client.withdraw_stream(&recipient, &stream_id);
        assert!(first > 0);
        let second = client.withdraw_stream(&recipient, &stream_id);
        assert_eq!(second, 0);
    }

    #[test]
    fn test_cancel_returns_correct_split() {
        let (env, sender, recipient, token, _, cid) = setup();
        let client = crate::StellarGrantsContractClient::new(&env, &cid);
        let stream_id = client.create_stream(&sender, &recipient, &1, &token, &100, &100);
        env.ledger().with_mut(|li| li.sequence_number += 30);
        let (sender_refund, recipient_payout) = client.cancel_stream(&sender, &stream_id);
        assert_eq!(recipient_payout, 3_000); // 30 * 100
        assert_eq!(sender_refund, 7_000); // 70 * 100
    }

    #[test]
    fn test_pause_resume_maintains_end_ledger() {
        let (env, sender, recipient, token, _, cid) = setup();
        let client = crate::StellarGrantsContractClient::new(&env, &cid);
        let stream_id = client.create_stream(&sender, &recipient, &1, &token, &1, &100);
        let stream_before = client.get_stream(&stream_id);
        let original_end = stream_before.end_ledger;

        env.ledger().with_mut(|li| li.sequence_number += 20);
        client.pause_stream(&sender, &stream_id);
        env.ledger().with_mut(|li| li.sequence_number += 10);
        client.resume_stream(&sender, &stream_id);

        let stream_after = client.get_stream(&stream_id);
        assert_eq!(stream_after.end_ledger, original_end + 10);
    }

    #[test]
    fn test_recipient_can_withdraw_accrued_balance_after_pause() {
        let (env, sender, recipient, token, _, cid) = setup();
        let client = crate::StellarGrantsContractClient::new(&env, &cid);
        let stream_id = client.create_stream(&sender, &recipient, &1, &token, &100, &100);

        env.ledger().with_mut(|li| li.sequence_number += 30);
        client.pause_stream(&sender, &stream_id);

        let withdrawn = client.withdraw_stream(&recipient, &stream_id);
        assert_eq!(withdrawn, 3_000);
    }

    #[test]
    fn test_create_stream_rejects_nonexistent_grant() {
        let (env, sender, recipient, token, _, cid) = setup();
        let client = crate::StellarGrantsContractClient::new(&env, &cid);
        // Grant 999 was never created.
        let result = client.try_create_stream(&sender, &recipient, &999, &token, &100, &100);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_stream_rejects_inactive_grant() {
        let (env, sender, recipient, token, _, cid) = setup();
        let client = crate::StellarGrantsContractClient::new(&env, &cid);
        env.as_contract(&cid, || {
            create_active_grant(&env, 2, &sender, &token, GrantStatus::Cancelled);
        });

        let result = client.try_create_stream(&sender, &recipient, &2, &token, &100, &100);
        assert!(result.is_err());
    }
}
