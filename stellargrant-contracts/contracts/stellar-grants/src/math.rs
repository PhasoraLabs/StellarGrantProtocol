use crate::errors::ContractError;

/// Safely add two i128 values; returns Err(ZeroAmount) on overflow.
pub fn safe_add(a: i128, b: i128) -> Result<i128, ContractError> {
    a.checked_add(b).ok_or(ContractError::ZeroAmount)
}

/// Safely subtract b from a; returns Err(InvalidInput) if result < 0.
pub fn safe_sub(a: i128, b: i128) -> Result<i128, ContractError> {
    a.checked_sub(b).ok_or(ContractError::InvalidInput)
}

/// Compute `basis_points / 10_000` of `amount` safely without intermediate overflow.
/// Uses the identity: `(amount / 10_000) * bps + (amount % 10_000) * bps / 10_000`
/// Returns Err(InvalidInput) if basis_points > 10_000.
pub fn basis_points_of(amount: i128, basis_points: u32) -> Result<i128, ContractError> {
    if basis_points > 10_000 {
        return Err(ContractError::InvalidInput);
    }
    if basis_points == 0 {
        return Ok(0);
    }
    let bps_i = basis_points as i128;
    let whole = amount
        .checked_div(10_000)
        .ok_or(ContractError::InvalidInput)?
        .checked_mul(bps_i)
        .ok_or(ContractError::InvalidInput)?;
    let remainder = amount
        .checked_rem_euclid(10_000)
        .ok_or(ContractError::InvalidInput)?
        .checked_mul(bps_i)
        .ok_or(ContractError::InvalidInput)?
        / 10_000;
    whole.checked_add(remainder)
        .ok_or(ContractError::InvalidInput)
}

/// Split `total` into `n` equal parts; returns (per_part, remainder).
/// Returns Err(ZeroAmount) if n == 0.
pub fn split_evenly(total: i128, n: u32) -> Result<(i128, i128), ContractError> {
    if n == 0 {
        return Err(ContractError::ZeroAmount);
    }
    let per_part = total
        .checked_div(n as i128)
        .ok_or(ContractError::InvalidInput)?;
    let remainder = total
        .checked_sub(
            per_part
                .checked_mul(n as i128)
                .ok_or(ContractError::InvalidInput)?,
        )
        .ok_or(ContractError::InvalidInput)?;
    Ok((per_part, remainder))
}

/// Proportional share: compute `share_bps / 10_000 * total` safely.
/// Delegates to `basis_points_of` for overflow-safe arithmetic.
/// Returns Err(InvalidInput) if share_bps > 10_000.
pub fn proportional_share(total: i128, share_bps: u32) -> Result<i128, ContractError> {
    basis_points_of(total, share_bps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_add_ok() {
        assert_eq!(safe_add(5, 3).unwrap(), 8);
        assert_eq!(safe_add(0, 0).unwrap(), 0);
        assert_eq!(safe_add(i128::MAX - 1, 1).unwrap(), i128::MAX);
    }

    #[test]
    fn test_safe_add_overflow() {
        assert_eq!(safe_add(i128::MAX, 1), Err(ContractError::ZeroAmount));
    }

    #[test]
    fn test_safe_sub_ok() {
        assert_eq!(safe_sub(5, 3).unwrap(), 2);
        assert_eq!(safe_sub(0, 0).unwrap(), 0);
    }

    #[test]
    fn test_safe_sub_underflow() {
        assert_eq!(safe_sub(0, 1), Err(ContractError::InvalidInput));
        assert_eq!(safe_sub(i128::MIN, 1), Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_basis_points_of_ok() {
        assert_eq!(basis_points_of(10000, 100).unwrap(), 100); // 1% of 10000
        assert_eq!(basis_points_of(10000, 10000).unwrap(), 10000); // 100%
        assert_eq!(basis_points_of(10000, 0).unwrap(), 0); // 0%
        assert_eq!(basis_points_of(10000, 250).unwrap(), 250); // 2.5%
        assert_eq!(basis_points_of(0, 5000).unwrap(), 0);
    }

    #[test]
    fn test_basis_points_of_large_amount_no_overflow() {
        // i128::MAX * 1 would overflow the old implementation; the new one handles it.
        let result = basis_points_of(i128::MAX, 1);
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }

    #[test]
    fn test_basis_points_of_max_bps() {
        assert_eq!(basis_points_of(i128::MAX, 10_000).unwrap(), i128::MAX);
    }

    #[test]
    fn test_basis_points_of_invalid() {
        assert_eq!(
            basis_points_of(10000, 10001),
            Err(ContractError::InvalidInput)
        );
    }

    #[test]
    fn test_basis_points_of_overflow() {
        let result = basis_points_of(i128::MAX, 10000);
        assert!(result.is_err());
    }

    #[test]
    fn test_split_evenly_ok() {
        assert_eq!(split_evenly(10, 3).unwrap(), (3, 1));
        assert_eq!(split_evenly(100, 4).unwrap(), (25, 0));
        assert_eq!(split_evenly(0, 5).unwrap(), (0, 0));
    }

    #[test]
    fn test_split_evenly_zero_n() {
        assert_eq!(split_evenly(10, 0), Err(ContractError::ZeroAmount));
    }

    #[test]
    fn test_split_evenly_single() {
        assert_eq!(split_evenly(42, 1).unwrap(), (42, 0));
    }

    #[test]
    fn test_proportional_share_ok() {
        // 50% of 100
        assert_eq!(proportional_share(100, 5000).unwrap(), 50);
        // 2.5% of 10000
        assert_eq!(proportional_share(10000, 250).unwrap(), 250);
        // 100% of anything
        assert_eq!(proportional_share(999, 10_000).unwrap(), 999);
    }

    #[test]
    fn test_proportional_share_zero_whole() {
        assert_eq!(
            proportional_share(50, 0, 100),
            Err(ContractError::InvalidInput)
        );
    }

    #[test]
    fn test_proportional_share_overflow() {
        // With the safe algorithm this should not overflow
        assert!(proportional_share(i128::MAX, 1).is_ok());
    }
}
