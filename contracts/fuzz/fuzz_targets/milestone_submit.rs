#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String, Vec};
use stellar_grants::{StellarGrantsContract, StellarGrantsContractClient};

// Fuzz target for milestone_submit. Every field below is derived from the
// fuzzer-supplied bytes so libFuzzer's coverage-guided mutation can actually
// steer program behavior (grant size, target index, and submitted text all vary).
fuzz_target!(|data: &[u8]| {
    // Need enough bytes to derive milestone count, grant id, index, and text from.
    if data.len() < 4 {
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
        let reviewers: Vec<Address> = Vec::new(&env);

        let num_milestones = (data[0] % 8) as u32 + 1;
        let grant_id = match client.try_grant_create(
            &owner,
            &title,
            &description,
            &token,
            &100,
            &10,
            &num_milestones,
            &reviewers,
        ) {
            Ok(Ok(id)) => id,
            _ => return,
        };

        // Occasionally target a nonexistent grant to exercise the GrantNotFound path.
        let target_grant_id = if data[1].is_multiple_of(4) {
            grant_id.wrapping_add(1 + data[1] as u64)
        } else {
            grant_id
        };

        // Occasionally target an out-of-range milestone index to exercise the
        // MilestoneIndexOutOfBounds path.
        let milestone_idx = (data[2] as u32) % (num_milestones + 2);

        // Split the remaining bytes between the description and proof_url so both
        // vary independently in content and length with the fuzzer input.
        let rest = &data[3..];
        let split = rest.len() / 2;
        let (desc_bytes, proof_bytes) = rest.split_at(split);
        let desc = String::from_str(&env, &bytes_to_text(desc_bytes));
        let proof = String::from_str(&env, &bytes_to_text(proof_bytes));

        let _ =
            client.try_milestone_submit(&target_grant_id, &milestone_idx, &owner, &desc, &proof);
    });
    if result.is_err() {
        panic!("Fuzz target panicked on non-empty input");
    }
});

/// Converts arbitrary fuzzer bytes into a bounded, valid UTF-8 string.
fn bytes_to_text(bytes: &[u8]) -> std::string::String {
    let bounded = &bytes[..bytes.len().min(256)];
    std::string::String::from_utf8_lossy(bounded).into_owned()
}
