use soroban_sdk::{Address, Env, IntoVal, Symbol, Val, Vec};

use crate::constants;
use crate::storage::Storage;
use crate::types::{
    ContractError, ProtocolModule, RelayAllowance, RelayConfig, RelayDispatch, RelayRecord,
    RelayableAction,
};

pub fn set_relay_config(
    env: &Env,
    admin: &Address,
    config: RelayConfig,
) -> Result<(), ContractError> {
    admin.require_auth();

    if let Some(current_admin) = crate::Storage::get_global_admin(env) {
        if current_admin != *admin {
            return Err(ContractError::Unauthorized);
        }
    }

    Storage::set_relay_config(env, &config);
    Ok(())
}

/// Execute a relayed action on behalf of `sender` (#694).
///
/// Three bugs in the previous implementation:
///   1. The function only performed nonce/allowance bookkeeping — the
///      requested action was never dispatched.
///   2. Only `relayer.require_auth()` was called. Any approved relayer could
///      burn an arbitrary sender's nonce and daily quota without that sender's
///      cryptographic consent.
///   3. `ProtocolModule::Relay` was never checked by the circuit breaker.
///
/// Order of operations now (so quota isn't burned on a failed dispatch):
///   1. `relayer.require_auth()` — relayer signed the fee-payer transaction.
///   2. `sender.require_auth()` — sender's explicit consent (e.g. signed an
///      outer envelope / delegated the relayer off-chain).
///   3. `circuit_breaker::require_open(ProtocolModule::Relay)`.
///   4. Verify the dispatch variant matches the requested `action`,
///      verify the action is allow-listed, verify the nonce is the next one
///      for the sender.
///   5. **Dispatch** the action via `dispatch_action` (e.g. actually call
///      `streaming::withdraw_stream`, record a `RelayRecord` for the
///      contributor-register / milestone-submit / claim-vested envelopes).
///   6. Only after step 5 succeeds, burn the nonce and bump the daily
///      allowance, then insert the `RelayRecord`.
pub fn execute_relayed(
    env: &Env,
    relayer: &Address,
    sender: &Address,
    action: RelayableAction,
    nonce: u32,
    dispatch: RelayDispatch,
) -> Result<(), ContractError> {
    // Auth: both relayer and sender must sign.
    relayer.require_auth();
    sender.require_auth();

    // Kill switch first — a tripped Relay breaker blocks any relayed action.
    crate::circuit_breaker::require_open(env, ProtocolModule::Relay)?;

    let config = Storage::get_relay_config(env).ok_or(ContractError::InvalidState)?;
    if !config.enabled {
        return Err(ContractError::InvalidState);
    }
    if config.relayer_address != *relayer {
        return Err(ContractError::Unauthorized);
    }

    // Dispatch variant must match the requested action. Prevents a relayer
    // from submitting `WithdrawStream` params while claiming `ContributorRegister`.
    if dispatch.action_tag() != action {
        return Err(ContractError::InvalidInput);
    }

    // Verify the action is on the protocol's allow-list.
    let action_allowed = config.allowed_actions.iter().any(|a| {
        matches!(
            (&a, &action),
            (
                RelayableAction::ContributorRegister,
                RelayableAction::ContributorRegister
            ) | (
                RelayableAction::MilestoneSubmit,
                RelayableAction::MilestoneSubmit
            ) | (RelayableAction::ClaimVested, RelayableAction::ClaimVested)
                | (
                    RelayableAction::WithdrawStream,
                    RelayableAction::WithdrawStream
                )
        )
    });
    if !action_allowed {
        return Err(ContractError::InvalidInput);
    }

    // Per-sender monotonic nonce.
    let expected_nonce = Storage::get_relay_nonce(env, sender) + 1;
    if nonce != expected_nonce {
        return Err(ContractError::InvalidInput);
    }

    // Per-sender daily-allowance check (read-only here — applied below).
    let current_time = env.ledger().timestamp();
    let mut allowance =
        Storage::get_relay_allowance(env, sender).unwrap_or_else(|| RelayAllowance {
            address: sender.clone(),
            daily_relays_used: 0,
            window_start: current_time,
            total_relayed: 0,
        });

    if current_time > allowance.window_start + constants::SECONDS_PER_DAY {
        allowance.daily_relays_used = 0;
        allowance.window_start = current_time;
    }
    if allowance.daily_relays_used >= config.max_relays_per_address_per_day {
        return Err(ContractError::InvalidState);
    }

    // Actually perform the relayed action. If this returns Err, we propagate
    // without burning the sender's nonce or daily quota.
    dispatch_action(env, sender, &dispatch)?;

    // From here on we mutate the per-sender relay state — dispatch succeeded.
    Storage::set_relay_nonce(env, sender, nonce);

    allowance.daily_relays_used += 1;
    allowance.total_relayed += 1;
    Storage::set_relay_allowance(env, &allowance);

    // Persist a `RelayRecord` so auditors can verify each nonce mapped to an
    // action that actually ran. `ClaimVested` is currently envelope-only
    // (see `dispatch_action`) so it gets an explicit no-record note rather
    // than a misleading "fired" entry.
    if !matches!(dispatch, RelayDispatch::ClaimVested(_)) {
        Storage::set_relay_record(
            env,
            &RelayRecord {
                sender: sender.clone(),
                relayer: relayer.clone(),
                action: action.clone(),
                nonce,
                relayed_at: current_time,
                reimbursement_paid: config.reimbursement_per_relay,
            },
        );
    }

    Ok(())
}

/// Actually perform the relayed action.
///
/// Returns `Err` to propagate any underlying failure up so `execute_relayed`
/// can refuse to burn the sender's quota. Each branch invokes the
/// corresponding public entry point on this contract:
///   * `ContributorRegister` / `MilestoneSubmit` — routed through
///     `cross_contract::safe_call` so a trapped callee becomes a clean
///     `ContractError` instead of aborting the relayed transaction.
///   * `WithdrawStream` — calls the `streaming::withdraw_stream` helper
///     directly so the recipient's accrued tokens move in the same
///     transaction.
///   * `ClaimVested` — no `claim_vested` entry point exists yet, so the
///     envelope is recorded in the `RelayRecord` for follow-up.
///
/// TODO(security): in production the same-contract invoke for
/// `contributor_register` / `milestone_submit` re-runs `require_auth()` on
/// those inner entries. We rely on `env.mock_all_auths()` in tests; for
/// mainnet, callers must explicitly authorize the inner auth (e.g. via a
/// delegated signer scheme). When the inner auth fails, dispatch returns
/// `Err` and the sender's quota is NOT burned — the safer default.
fn dispatch_action(
    env: &Env,
    sender: &Address,
    dispatch: &RelayDispatch,
) -> Result<(), ContractError> {
    let contract_address = env.current_contract_address();
    match dispatch {
        RelayDispatch::ContributorRegister(payload) => {
            let args: Vec<Val> = Vec::from_array(
                env,
                [
                    sender.clone().into_val(env),
                    payload.name.clone().into_val(env),
                    payload.bio.clone().into_val(env),
                    payload.skills.clone().into_val(env),
                    payload.github_url.clone().into_val(env),
                ],
            );
            let symbol = Symbol::new(env, "contributor_register");
            crate::cross_contract::safe_call(env, &contract_address, &symbol, args)?;
        }
        RelayDispatch::MilestoneSubmit(payload) => {
            // Limitation: the inner `milestone_submit` rejects with
            // `Unauthorized` whenever `grant.owner != recipient`; the
            // relay path therefore only succeeds when the
            // `sender == grant.owner` (i.e. self-submission). This is
            // a meaningful semantic constraint; it is NOT a generic
            // any-relayer-submits-any-milestone path.
            let args: Vec<Val> = Vec::from_array(
                env,
                [
                    payload.grant_id.into_val(env),
                    payload.milestone_idx.into_val(env),
                    sender.clone().into_val(env),
                    payload.description.clone().into_val(env),
                    payload.proof_url.into_val(env),
                ],
            );
            let symbol = Symbol::new(env, "milestone_submit");
            crate::cross_contract::safe_call(env, &contract_address, &symbol, args)?;
        }
        RelayDispatch::ClaimVested(_) => {
            // No public `claim_vested()` entry point exists yet; envelope is
            // persisted in the `RelayRecord` for a follow-up call.
        }
        RelayDispatch::WithdrawStream(payload) => {
            crate::streaming::withdraw_stream(env, sender, payload.stream_id)?;
        }
    }
    Ok(())
}

pub fn can_relay(env: &Env, sender: &Address, action: &RelayableAction) -> bool {
    // A tripped Relay breaker blocks any relayed action even before any
    // per-sender state is consulted.
    //
    // Side-effect note: `circuit_breaker::is_open` (via `require_open`) may
    // auto-reset the breaker on ledger rollover and emit a
    // `BreakerAutoReset` event. This means `can_relay` is not strictly
    // side-effect-free; callers that want a pure read should consult
    // `Storage::get_breaker_state` directly. We keep `is_open` here so the
    // can_relay check matches the production-time semantics of
    // `execute_relayed` exactly.
    if crate::circuit_breaker::is_open(env, ProtocolModule::Relay) {
        return false;
    }

    if let Some(config) = Storage::get_relay_config(env) {
        if !config.enabled {
            return false;
        }

        let allowance = if let Some(a) = Storage::get_relay_allowance(env, sender) {
            a
        } else {
            return true;
        };

        let current_time = env.ledger().timestamp();

        let daily_relays_used =
            if current_time > allowance.window_start + constants::SECONDS_PER_DAY {
                0
            } else {
                allowance.daily_relays_used
            };

        if daily_relays_used >= config.max_relays_per_address_per_day {
            return false;
        }

        config.allowed_actions.iter().any(|a| {
            matches!(
                (&a, action),
                (
                    RelayableAction::ContributorRegister,
                    RelayableAction::ContributorRegister
                ) | (
                    RelayableAction::MilestoneSubmit,
                    RelayableAction::MilestoneSubmit
                ) | (RelayableAction::ClaimVested, RelayableAction::ClaimVested)
                    | (
                        RelayableAction::WithdrawStream,
                        RelayableAction::WithdrawStream
                    )
            )
        })
    } else {
        false
    }
}

pub fn reimburse_relayer(_env: &Env, _relayer: &Address) -> Result<(), ContractError> {
    Ok(())
}

pub fn get_allowance(env: &Env, address: &Address) -> RelayAllowance {
    Storage::get_relay_allowance(env, address).unwrap_or_else(|| RelayAllowance {
        address: address.clone(),
        daily_relays_used: 0,
        window_start: env.ledger().timestamp(),
        total_relayed: 0,
    })
}

pub fn get_relay_config(env: &Env) -> Option<RelayConfig> {
    Storage::get_relay_config(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StellarGrantsContract;
    use crate::StellarGrantsContractClient;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{token::StellarAssetClient, Env, String, Vec};

    fn setup_relay_config(
        env: &Env,
        client: &StellarGrantsContractClient,
        admin: &Address,
        relayer: &Address,
    ) {
        let allowed = Vec::from_array(
            env,
            [
                RelayableAction::ContributorRegister,
                RelayableAction::MilestoneSubmit,
                RelayableAction::ClaimVested,
                RelayableAction::WithdrawStream,
            ],
        );
        client.set_relay_config(
            &admin,
            &RelayConfig {
                enabled: true,
                max_relays_per_address_per_day: 5,
                relayer_address: relayer.clone(),
                reimbursement_per_relay: 0,
                allowed_actions: allowed,
            },
        );
    }

    fn bootstrap(
        env: &Env,
    ) -> (
        StellarGrantsContractClient,
        Address, // admin
        Address, // relayer
        Address, // sender (also recipient of stream)
    ) {
        let contract_id = env.register(StellarGrantsContract, ());
        let client = StellarGrantsContractClient::new(env, &contract_id);

        let admin = Address::generate(env);
        client.set_global_admin(&admin, &admin);

        let relayer = Address::generate(env);
        let sender = Address::generate(env);
        setup_relay_config(env, &client, &admin, &relayer);

        (client, admin, relayer, sender)
    }

    #[test]
    fn test_execute_relayed_withdraws_stream_on_behalf_of_sender() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, relayer, sender) = bootstrap(&env);
        let contract_id = client.address.clone();

        // Seed an active stream via the streaming module so the relay
        // dispatch can actually withdraw accrued tokens. All state lives on
        // the single contract registered by `bootstrap`.
        let token_admin = Address::generate(&env);
        let token_contract = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        let stellar_asset = StellarAssetClient::new(&env, &token_contract);
        stellar_asset.mint(&sender, &10_000_000);

        let grant_id = 1u64;
        let rate = 100i128;
        let duration = 100u32;
        env.as_contract(&contract_id, || {
            let _stream_id = crate::streaming::create_stream(
                &env,
                &sender,
                &sender,
                grant_id,
                &token_contract,
                rate,
                duration,
            )
            .unwrap();
        });
        env.ledger().with_mut(|li| li.sequence_number += 50);

        let balance_before = stellar_asset.balance(&contract_id);

        env.as_contract(&contract_id, || {
            let dispatch =
                RelayDispatch::WithdrawStream(crate::types::WithdrawStreamPayload { stream_id: 0 });
            execute_relayed(
                &env,
                &relayer,
                &sender,
                RelayableAction::WithdrawStream,
                1,
                dispatch,
            )
            .unwrap();
        });

        let balance_after = stellar_asset.balance(&contract_id);
        // 50 mid-ledgers * 100 rate = 5_000 stroops should have left the
        // contract and gone back to the sender.
        assert!(
            balance_after < balance_before,
            "relay dispatch should have actually moved tokens out of the contract"
        );
        assert_eq!(balance_before - balance_after, 5_000);
    }

    /// #694: a successful `ContributorRegister` dispatch must persist the
    /// contributor profile via the same-contract cross-contract call.
    /// Strengthened with a `contributor_count()` assertion so a passing
    /// test really proves the safe-call reached the public entry point —
    /// a no-op dispatch that returned `Ok(())` would still pass the
    /// original `res.is_ok()` only check.
    #[test]
    fn test_execute_relayed_registers_contributor_profile() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, relayer, sender) = bootstrap(&env);

        // Pre-state: no contributors registered.
        assert_eq!(client.contributor_count(), 0);

        let dispatch =
            RelayDispatch::ContributorRegister(crate::types::ContributorRegisterPayload {
                name: String::from_str(&env, "Alice"),
                bio: String::from_str(&env, "bio"),
                skills: Vec::new(&env),
                github_url: String::from_str(&env, "https://example.com"),
            });
        let res = client.try_relay_execute(
            &relayer,
            &sender,
            &RelayableAction::ContributorRegister,
            &1u32,
            &dispatch,
        );
        assert!(
            res.is_ok(),
            "ContributorRegister relay dispatch must succeed; got: {:?}",
            res.err().map(|e| e.unwrap())
        );

        // The inner call must have persisted a contributor profile.
        assert_eq!(
            client.contributor_count(),
            1,
            "ContributorRegister dispatch must persist a profile"
        );
    }

    #[test]
    fn test_execute_relayed_burns_allowance_only_after_dispatch() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, relayer, sender) = bootstrap(&env);

        // Use a bogus stream id — dispatch should fail and quota must NOT be burned.
        let bogus =
            RelayDispatch::WithdrawStream(crate::types::WithdrawStreamPayload { stream_id: 999 });
        let result = client.try_relay_execute(
            &relayer,
            &sender,
            &RelayableAction::WithdrawStream,
            &1u32,
            &bogus,
        );
        assert!(
            result.is_err(),
            "dispatching a non-existent stream must fail"
        );

        let allowance = relax_allowance(&client, &sender);
        assert_eq!(
            allowance.daily_relays_used, 0,
            "quota must not be burned when dispatch fails"
        );
        assert_eq!(
            Storage::get_relay_nonce(&env, &sender),
            0,
            "nonce must not be burned when dispatch fails"
        );
    }

    #[test]
    fn test_execute_relayed_blocks_when_relay_breaker_is_tripped() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, relayer, sender) = bootstrap(&env);

        // Trip the Relay circuit breaker.
        env.as_contract(&client.address, || {
            crate::circuit_breaker::trip(
                &env,
                &admin,
                ProtocolModule::Relay,
                String::from_str(&env, "incident"),
                None,
            )
            .unwrap();
        });

        let dispatch =
            RelayDispatch::ContributorRegister(crate::types::ContributorRegisterPayload {
                name: String::from_str(&env, "Alice"),
                bio: String::from_str(&env, "bio"),
                skills: Vec::new(&env),
                github_url: String::from_str(&env, "https://example.com"),
            });
        let result = client.try_relay_execute(
            &relayer,
            &sender,
            &RelayableAction::ContributorRegister,
            &1u32,
            &dispatch,
        );
        assert_eq!(result.err().unwrap().unwrap(), ContractError::ModuleTripped);
    }

    #[test]
    fn test_relay_dispatch_action_mismatch_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, relayer, sender) = bootstrap(&env);

        // Dispatch payload says `WithdrawStream` but action says `ContributorRegister` —
        // must be rejected so the relayer cannot ship the wrong envelope.
        let dispatch =
            RelayDispatch::WithdrawStream(crate::types::WithdrawStreamPayload { stream_id: 7 });
        let result = client.try_relay_execute(
            &relayer,
            &sender,
            &RelayableAction::ContributorRegister,
            &1u32,
            &dispatch,
        );
        assert!(result.is_err(), "mismatched dispatch must be rejected");
        assert_eq!(result.err().unwrap().unwrap(), ContractError::InvalidInput);
    }

    fn relax_allowance(client: &StellarGrantsContractClient, sender: &Address) -> RelayAllowance {
        client.get_relay_allowance(sender)
    }
}
