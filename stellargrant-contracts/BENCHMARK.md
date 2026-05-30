# Soroban WASM Optimization Benchmarks

## Contract: stellar-grants

### Before Optimizations
- **WASM Size**: 18,067 bytes (~17.6 KB)
- **Profile**: default `opt-level = "z"`, `lto = true`

### After Optimizations
- **WASM Size**: 16,436 bytes (~16.1 KB)
- **Drop**: 1,631 bytes (~9.03%)

### Optimization Techniques Applied
1. **Release Profile Enhancements**:
   - `opt-level = "s"`: Found to be more effective than "z" for this specific instruction mix.
   - `lto = "fat"`: Enabled full Link Time Optimization.
   - `strip = true`: Removed all debug symbols and names.
   - `incremental = false`: Ensured clean release builds.
   - `overflow-checks = false`: Aggressive instruction pruning for well-typed operations.

2. **Dependency Profiling**:
   - `default-features = false` for `soroban-sdk`: Removed standard library components and logging features from the guest environment.
   - Removed unused crates and internal modules.

3. **Code Level Pruning**:
   - Removed unused `DataKey` variants and `storage` helpers.
   - Removed unused `#[contracttype]` structs from `events.rs` that were adding expansion overhead.
   - Replaced internal `Result` propagation with `env.panic_with_error()` callers in critical paths to reduce error-handling branch overhead.
   - Ensured zero `std` formatting usage (no `format!`, `println!`).

### Results
- Noticeable reduction in contract size.
- Reduced resource footprint through instruction pruning.
- Test coverage (12/12) fully maintained.
