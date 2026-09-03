use soroban_sdk::{
    testutils::{Address as TestAddress, Ledger},
    Address, Env, String, Vec,
};
use stellar_grants::{ContractVersion, StellarGrantsContractClient};

fn setup(env: &Env) -> (StellarGrantsContractClient<'_>, Address) {
    let contract_id = env.register_contract(None, stellar_grants::StellarGrantsContract);
    let client = StellarGrantsContractClient::new(env, &contract_id);
    let deployer = <Address as TestAddress>::generate(env);
    (client, deployer)
}

/// Helper: build a ContractVersion value.
fn make_version(
    env: &Env,
    major: u32,
    minor: u32,
    patch: u32,
    deployer: &Address,
) -> ContractVersion {
    ContractVersion {
        major,
        minor,
        patch,
        deployed_at: env.ledger().timestamp(),
        deployer: deployer.clone(),
    }
}

#[test]
fn test_initialize_sets_version_1_0_0() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, deployer) = setup(&env);

    client.initialize(&deployer);

    let version = client
        .get_contract_version()
        .expect("version should be set after initialize");
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 0);
    assert_eq!(version.patch, 0);
    assert_eq!(version.deployer, deployer);
}

#[test]
fn test_initialize_is_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, deployer) = setup(&env);

    // First call sets the version
    client.initialize(&deployer);
    let v1 = client.get_contract_version().unwrap();

    // Second call is a no-op; version unchanged
    let other = <Address as TestAddress>::generate(&env);
    client.initialize(&other);
    let v2 = client.get_contract_version().unwrap();

    assert_eq!(v1.major, v2.major);
    assert_eq!(v1.minor, v2.minor);
    assert_eq!(v1.patch, v2.patch);
    assert_eq!(v1.deployer, deployer);
}

#[test]
fn test_migration_v1_to_v2_records_history() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, deployer) = setup(&env);

    client.initialize(&deployer);
    client.set_global_admin(&deployer, &deployer);

    let target = make_version(&env, 2, 0, 0, &deployer);
    let record = client.run_migration(&deployer, &target);

    assert!(record.success);
    assert_eq!(record.from_version, 1);
    assert_eq!(record.to_version, 2);

    let history = client.migration_history();
    assert_eq!(history.len(), 1);
    let logged = history.get(0).unwrap();
    assert_eq!(logged.from_version, 1);
    assert_eq!(logged.to_version, 2);
    assert!(logged.success);
}

#[test]
fn test_migration_updates_stored_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, deployer) = setup(&env);

    client.initialize(&deployer);
    client.set_global_admin(&deployer, &deployer);

    let target = make_version(&env, 2, 1, 0, &deployer);
    client.run_migration(&deployer, &target);

    let version = client.get_contract_version().unwrap();
    assert_eq!(version.major, 2);
    assert_eq!(version.minor, 1);
    assert_eq!(version.patch, 0);
}

#[test]
fn test_migration_idempotent_noop_when_already_at_target() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, deployer) = setup(&env);

    client.initialize(&deployer);
    client.set_global_admin(&deployer, &deployer);

    // Migrate to v2
    let target = make_version(&env, 2, 0, 0, &deployer);
    client.run_migration(&deployer, &target);
    assert_eq!(client.migration_history().len(), 1);

    // Run the same migration again — idempotent no-op
    let target2 = make_version(&env, 2, 0, 0, &deployer);
    let record = client.run_migration(&deployer, &target2);
    assert!(record.success);
    assert_eq!(
        record.notes,
        String::from_str(&env, "no-op: already at target version")
    );

    // History must not grow for no-ops
    assert_eq!(client.migration_history().len(), 1);
}

#[test]
fn test_migration_state_continuity_contributor_survives_upgrade() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, deployer) = setup(&env);

    client.initialize(&deployer);
    client.set_global_admin(&deployer, &deployer);

    // Register a contributor under v1 to establish pre-migration state
    let contributor = <Address as TestAddress>::generate(&env);
    client.contributor_register(
        &contributor,
        &String::from_str(&env, "Alice"),
        &String::from_str(&env, "Rust developer"),
        &Vec::new(&env),
        &String::from_str(&env, "https://github.com/alice"),
    );

    let pre_migration_count = client.contributor_count();
    assert_eq!(pre_migration_count, 1);

    // Run migration to v2
    let target = make_version(&env, 2, 0, 0, &deployer);
    let record = client.run_migration(&deployer, &target);
    assert!(record.success);

    // Contributor registry must survive the migration (state continuity)
    let post_migration_count = client.contributor_count();
    assert_eq!(
        post_migration_count, 1,
        "contributor count must be unchanged after migration"
    );

    // Version is correctly updated
    let new_version = client.get_contract_version().unwrap();
    assert_eq!(new_version.major, 2);
}

#[test]
fn test_migration_unauthorized_when_not_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, deployer) = setup(&env);

    client.initialize(&deployer);
    // Set a different admin
    let admin = <Address as TestAddress>::generate(&env);
    client.set_global_admin(&deployer, &admin);

    // deployer is no longer the admin; migration should fail
    let target = make_version(&env, 2, 0, 0, &deployer);
    let result = client.try_run_migration(&deployer, &target);
    assert!(result.is_err());
}
