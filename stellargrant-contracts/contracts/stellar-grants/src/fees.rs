use soroban_sdk::{token, Address, Env};

use crate::events::Events;
use crate::storage::Storage;
use crate::types::ContractError;

/// Compute protocol fee from gross amount. Used in tests for fee calculation validation.
#[allow(dead_code)]
pub fn compute_fee(gross: i128, fee_bps: u32) -> Result<i128, ContractError> {
    if fee_bps == 0 || gross <= 0 {
        return Ok(0);
    }
    crate::math::basis_points_of(gross, fee_bps)
}

pub fn total_fees_collected(env: &Env, token: &Address) -> i128 {
    Storage::get_fees_collected(env, token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::Env;

    #[test]
    fn test_compute_fee_zero_bps() {
        let result = compute_fee(1_000_000, 0).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_compute_fee_one_percent() {
        let result = compute_fee(1_000_000, 100).unwrap();
        assert_eq!(result, 10_000);
    }

    #[test]
    fn test_compute_fee_large_amount() {
        let result = compute_fee(100_000_000, 250).unwrap();
        assert_eq!(result, 2_500_000);
    }

    #[test]
    fn test_compute_fee_negative_gross_returns_zero() {
        let result = compute_fee(-1, 100).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_fee_accumulation_across_calls() {
        assert_eq!(compute_fee(1_000_000, 100).unwrap(), 10_000);
        assert_eq!(compute_fee(2_000_000, 100).unwrap(), 20_000);
    }
}
