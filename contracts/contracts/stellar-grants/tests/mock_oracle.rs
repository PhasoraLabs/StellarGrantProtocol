use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Ledger},
    Address, Env,
};
use stellar_grants::{OracleConfig, StellarGrantsContract, StellarGrantsContractClient};

/// Simple mock oracle exposing `price(token) -> Option<(i128, u64)>`.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Price(Address),
}

#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn set_price(env: Env, token: Address, price: i128, timestamp: u64) {
        env.storage()
            .instance()
            .set(&DataKey::Price(token), &(price, timestamp));
    }

    pub fn price(env: Env, token: Address) -> Option<(i128, u64)> {
        env.storage().instance().get(&DataKey::Price(token))
    }
}

const PRICE_SCALE: i128 = 10_000_000;

#[test]
fn test_convert_amount_xlm_usdc_roundtrip() {
    let env = Env::default();
    env.mock_all_auths();

    let grants_id = env.register(StellarGrantsContract, ());
    let grants = StellarGrantsContractClient::new(&env, &grants_id);

    let admin = Address::generate(&env);
    grants.set_global_admin(&admin, &admin);

    let oracle_id = env.register(MockOracle, ());
    let oracle = MockOracleClient::new(&env, &oracle_id);

    let usdc = Address::generate(&env);
    let xlm = Address::generate(&env);
    let now = 5_000u64;
    env.ledger().set_timestamp(now);

    oracle.set_price(&xlm, &12_000_000, &now);
    oracle.set_price(&usdc, &PRICE_SCALE, &now);

    grants.set_oracle(
        &admin,
        &OracleConfig {
            oracle_address: oracle_id.clone(),
            base_token: usdc.clone(),
            staleness_threshold: 3_600,
        },
    );

    let xlm_amount = 100_000_000i128;
    let usdc_amount = grants.convert_amount(&xlm_amount, &xlm, &usdc);
    assert_eq!(usdc_amount, 120_000_000);

    let xlm_back = grants.convert_amount(&usdc_amount, &usdc, &xlm);
    assert_eq!(xlm_back, xlm_amount);
}

#[test]
fn test_stale_price_returns_err_from_get_price() {
    let env = Env::default();
    env.mock_all_auths();

    let grants_id = env.register(StellarGrantsContract, ());
    let grants = StellarGrantsContractClient::new(&env, &grants_id);

    let admin = Address::generate(&env);
    grants.set_global_admin(&admin, &admin);

    let oracle_id = env.register(MockOracle, ());
    let oracle = MockOracleClient::new(&env, &oracle_id);

    let usdc = Address::generate(&env);
    oracle.set_price(&usdc, &PRICE_SCALE, &1_000);

    grants.set_oracle(
        &admin,
        &OracleConfig {
            oracle_address: oracle_id.clone(),
            base_token: usdc.clone(),
            staleness_threshold: 100,
        },
    );

    env.ledger().set_timestamp(10_000);
    let err = grants.try_get_price(&usdc);
    assert!(err.is_err());
}
