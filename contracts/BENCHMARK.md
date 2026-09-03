# Soroban WASM Optimization Benchmarks

## Contract: stellar-grants

Measured on this branch with **Stellar CLI 27.0.0** and **Rust 1.97.1**, using the
workspace release profile in `contracts/Cargo.toml`.

> **Note on `stellar contract build`:** Stellar CLI 27 rejects
> `overflow-checks = false` in `[profile.release]`. The figures below were
> produced with a temporary `overflow-checks = true` so the documented
> `stellar contract build` command could run; the checked-in workspace profile
> still uses `overflow-checks = false` for size. Plain
> `cargo build --target wasm32v1-none --release` (without the CLI’s spec-shaking
> env) produces a larger artifact (~700 KB) and is not the size tracked here.

### Build commands

```bash
cd contracts
stellar contract build --package stellar-grants --locked --optimize=false
stellar contract build --package stellar-grants --locked --optimize
```

`stellar contract build` sets `SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1`,
which is required for the sizes below (plain `cargo build` alone will not match).

### Results

| Build | Size |
|------|------|
| Release (`--optimize=false`, with spec shaking) | `511,030` bytes (~499.1 KB) |
| Optimized (`--optimize`) | `447,150` bytes (~436.7 KB) |
| Optimizer delta | `63,880` bytes smaller (`12.5%`) |

### Release profile

From `contracts/Cargo.toml`:

```toml
[profile.release]
opt-level = "s"
overflow-checks = false
debug = 0
strip = true
debug-assertions = false
panic = "abort"
codegen-units = 1
lto = "fat"
incremental = false
```

### Optimization techniques applied

1. **Release profile** — `opt-level = "s"`, fat LTO, symbol stripping, single codegen unit, abort-on-panic.
2. **Spec shaking** — `stellar contract build` enables Soroban SDK spec shaking so unused contract-spec / rustdoc payload is dropped from the WASM.
3. **WASM optimizer** — `stellar contract build --optimize` (wasm-opt style pass) removes an additional ~12.5% from the release artifact.

### Tests

`cargo test --package stellar-grants` **does not currently compile** (23 errors as of
this measurement — e.g. tests calling missing client methods such as
`reviewer_get_sla` / `check_reviewer_sla`). A pass/fail coverage claim is therefore
not reported.

In-tree `#[test]` attributes under `contracts/stellar-grants`: **415** defined tests
(awaiting a green suite before a runnable count can be published).
