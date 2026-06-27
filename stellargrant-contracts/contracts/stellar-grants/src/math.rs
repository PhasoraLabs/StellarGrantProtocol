use crate::errors::ContractError;

/// Safely add two i128 values; returns Err(ZeroAmount) on overflow.
pub fn safe_add(a: i128, b: i128) -> Result<i128, ContractError> {
    a.checked_add(b).ok_or(ContractError::ZeroAmount)
}

/// Safely subtract b from a; returns Err(InvalidInput) if result < 0.
pub fn safe_sub(a: i128, b: i128) -> Result<i128, ContractError> {
    a.checked_sub(b).ok_or(ContractError::InvalidInput)
}

/// Calculate `basis_points / 10_000` of `amount` (e.g. 250 bps = 2.5%).
/// Returns Err(InvalidInput) if basis_points > 10_000.
pub fn basis_points_of(amount: i128, basis_points: u32) -> Result<i128, ContractError> {
    if basis_points > 10_000 {
        return Err(ContractError::InvalidInput);
    }
    amount
        .checked_mul(basis_points as i128)
        .ok_or(ContractError::InvalidInput)?
        .checked_div(10_000)
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

/// Proportional share: (part / whole) * scale. Used for reviewer fee splits.
/// Returns Err(InvalidInput) if whole == 0.
pub fn proportional_share(part: i128, whole: i128, scale: i128) -> Result<i128, ContractError> {
    if whole == 0 {
        return Err(ContractError::InvalidInput);
    }
    part.checked_mul(scale)
        .ok_or(ContractError::InvalidInput)?
        .checked_div(whole)
        .ok_or(ContractError::InvalidInput)
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
        // Large amount * large bps should not panic
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
        // 50 is 50% of 100, scale 100 => 50
        assert_eq!(proportional_share(50, 100, 100).unwrap(), 50);
        // 25 is 25% of 100, scale 1000 => 250
        assert_eq!(proportional_share(25, 100, 1000).unwrap(), 250);
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
        assert!(proportional_share(i128::MAX, 1, 2).is_err());
    }
}
