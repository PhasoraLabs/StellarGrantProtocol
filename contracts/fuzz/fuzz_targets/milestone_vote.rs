#![no_main]
use libfuzzer_sys::fuzz_target;
use soroban_sdk::{Address, Env, String as SorobanString, Vec};
use stellar_grants::{StellarGrantsContract, Storage};

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    let env = Env::default();

    let owner = Address::from_str(
        &env,
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
    );
    let token = Address::from_str(
        &env,
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHG",
    );
    let reviewer = Address::from_str(
        &env,
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHJ",
    );
    let mut reviewers: Vec<Address> = Vec::new(&env);
    reviewers.push_back(reviewer.clone());

    let milestone_amount: i128 = 10;
    let total_milestones: u32 = 1;
    let grant_id = match StellarGrantsContract::grant_create(
        env.clone(),
        owner.clone(),
        SorobanString::from_str(&env, "Fuzz Grant"),
        SorobanString::from_str(&env, "Fuzzing"),
        token.clone(),
        100i128,
        milestone_amount,
        total_milestones,
        reviewers.clone(),
    ) {
        Ok(id) => id,
        Err(_) => return,
    };

    let milestone_idx = data[1] as u32 % 4;
    let _ = StellarGrantsContract::milestone_submit(
        env.clone(),
        grant_id,
        milestone_idx,
        owner.clone(),
        SorobanString::from_str(&env, "desc"),
        SorobanString::from_str(&env, "proof"),
    );

    let approve = data[0].is_multiple_of(2);
    let _ = StellarGrantsContract::milestone_vote(
        env.clone(),
        grant_id,
        milestone_idx,
        reviewer.clone(),
        approve,
        None,
    );

    if let Some(milestone) = Storage::get_milestone(&env, grant_id, milestone_idx) {
        let count = milestone
            .votes
            .iter()
            .filter(|(v, _)| v == &reviewer)
            .count();
        assert!(count <= 1, "Double vote invariant violated!");
    }
});
