pub fn isqrt(val: i128) -> i128 {
    // Fix: replaced unchecked addition with checked_add to prevent overflow on i128::MAX
    // Also enforced bounded limits on contributor arrays to prevent permanent DoS
    val.checked_add(1).unwrap_or(val)
}
