#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String, Vec};
use stellar_grants::{StellarGrantsContract, StellarGrantsContractClient};

// Fuzz target for grant_create and grant_fund lifecycle
fuzz_target!(|data: &[u8]| {
    // Skip if input is empty
    if data.is_empty() {
        return;
    }
    let result = std::panic::catch_unwind(|| {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let client = StellarGrantsContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let title = String::from_str(&env, "Fuzz Grant");
        let description = String::from_str(&env, "Fuzzing");
        let token = Address::generate(&env);
        let total_amount = i128::from_le_bytes([data.first().copied().unwrap_or(1); 16]);
        let milestone_amount = i128::from_le_bytes([data.get(1).copied().unwrap_or(1); 16]);
        let num_milestones = (data.get(2).copied().unwrap_or(1) % 10) as u32 + 1;
        let reviewers: Vec<Address> = Vec::new(&env);

        let grant_id = match client.try_grant_create(
            &owner,
            &title,
            &description,
            &token,
            &total_amount,
            &milestone_amount,
            &num_milestones,
            &reviewers,
        ) {
            Ok(Ok(id)) => id,
            _ => return,
        };

        let funder = Address::generate(&env);
        let fund_amount = i128::from_le_bytes([data.get(3).copied().unwrap_or(1); 16]);
        let _ = client.try_grant_fund(&grant_id, &funder, &fund_amount);

        // Invariant check: escrowed funds == sum of unapproved milestone amounts
        if let Ok(Ok(grant)) = client.try_get_grant(&grant_id) {
            let mut sum_unapproved: i128 = 0;
            for idx in 0..grant.total_milestones {
                if let Ok(Ok(milestone)) = client.try_get_milestone(&grant_id, &idx) {
                    use stellar_grants::MilestoneState;
                    if milestone.state != MilestoneState::Approved
                        && milestone.state != MilestoneState::Paid
                    {
                        sum_unapproved = sum_unapproved.saturating_add(milestone.amount);
                    }
                }
            }
            // Only check if milestones exist and grant is active
            if grant.total_milestones > 0 && grant.status == stellar_grants::GrantStatus::Active {
                assert!(grant.escrow_balance >= 0, "Escrow balance negative");
                assert!(sum_unapproved >= 0, "Sum of unapproved milestones negative");
                if grant.escrow_balance != sum_unapproved {
                    panic!(
                        "escrow_balance {} != sum_unapproved {} for grant {}",
                        grant.escrow_balance, sum_unapproved, grant.id
                    );
                }
            }
        }
    });
    // If a panic occurred, treat as a fuzz failure only if input was not obviously invalid
    if result.is_err() && !data.is_empty() {
        panic!("Fuzz target panicked on non-empty input");
    }
});
