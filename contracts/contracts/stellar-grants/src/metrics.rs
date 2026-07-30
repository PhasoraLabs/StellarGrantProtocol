use crate::errors::Error;
use crate::storage::keys::StorageKey;
use soroban_sdk::Env;

#[derive(Clone)]
pub struct ProtocolMetrics {
    pub total_bounties_created: u64,
    pub total_bounties_awarded: u64,
}

pub struct MetricsModule;

impl MetricsModule {
    pub fn increment_bounties_created(env: &Env) -> Result<(), Error> {
        let key = StorageKey::Metrics;
        let mut metrics: ProtocolMetrics = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(ProtocolMetrics {
                total_bounties_created: 0,
                total_bounties_awarded: 0,
            });
        metrics.total_bounties_created += 1;
        env.storage().persistent().set(&key, &metrics);
        Ok(())
    }

    pub fn increment_bounties_awarded(env: &Env) -> Result<(), Error> {
        let key = StorageKey::Metrics;
        let mut metrics: ProtocolMetrics = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(ProtocolMetrics {
                total_bounties_created: 0,
                total_bounties_awarded: 0,
            });
        metrics.total_bounties_awarded += 1;
        env.storage().persistent().set(&key, &metrics);
        Ok(())
    }

    pub fn get_metrics(env: &Env) -> ProtocolMetrics {
        let key = StorageKey::Metrics;
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(ProtocolMetrics {
                total_bounties_created: 0,
                total_bounties_awarded: 0,
            })
    }
}
