# Code Coverage

This document describes the code coverage setup for the `contracts` workspace. Coverage is currently a **local-only** workflow — it is not yet wired into CI (see [CI Integration](#ci-integration)).

---

## Overview

Coverage is measured using [`cargo-tarpaulin`](https://github.com/xd009642/tarpaulin) with the LLVM engine, targeting production contract logic only. Test files are excluded from all measurements.

- **Engine**: LLVM (`llvm-tools-preview`)
- **Scope**: `lib` targets across the workspace only
- **Excluded**: `*test.rs`, `*tests.rs`, `tests/*`
- **Output**: `cobertura.xml` (Cobertura/Codecov format)

---

## Configuration

**`.tarpaulin.toml`** (located in `contracts/`):

```toml
[config]
exclude-files = ["*test.rs", "*tests.rs", "tests/*"]
ignore-tests = true
lib = true
workspace = true
out = ["Xml"]
engine = "llvm"
timeout = "120s"
```

---

## Running Coverage Locally

### Prerequisites

```bash
cargo install cargo-tarpaulin
```

### Run

```bash
cd contracts
cargo tarpaulin --workspace --lib --target x86_64-unknown-linux-gnu --engine llvm --out Xml
```

> **Note**: The `.tarpaulin.toml` config is auto-detected. This is currently a local-only workflow — see [CI Integration](#ci-integration) below.

### What is measured

| File | Included |
|---|---|
| `contracts/stellar-grants/src/lib.rs` | ✅ Yes |
| `contracts/stellar-grants/src/storage/mod.rs` | ✅ Yes |
| `contracts/stellar-grants/src/storage/helpers.rs` | ✅ Yes |
| `contracts/stellar-grants/src/storage/keys.rs` | ✅ Yes |
| `contracts/stellar-grants/src/types.rs` | ✅ Yes |
| `contracts/stellar-grants/src/events.rs` | ✅ Yes |
| `contracts/stellar-grants/src/test.rs` | ❌ Excluded |

---

## CI Integration

> ⚠️ **Coverage is not currently wired into CI.** `.github/workflows/ci.yml` has no
> tarpaulin step and no Codecov upload — the `contracts` job only runs
> `cargo fmt --check`, `cargo clippy`, `cargo check`, and `cargo test`, all
> against the WASM target. The `.tarpaulin.toml` config above exists and is
> valid, but nothing in CI currently invokes it. Coverage is a **local-only**
> workflow for now; run the command above yourself before opening a PR if you
> want a coverage number.

If someone wires this up in the future, a coverage job would need to:

1. Set up Rust with the `llvm-tools-preview` component
2. Cache dependencies (e.g. with `Swatinem/rust-cache`)
3. Install `cargo-tarpaulin`
4. Run coverage on the **native host target** (`x86_64-unknown-linux-gnu`) — not WASM
5. Upload `cobertura.xml` as a build artifact and/or to a coverage service such as [Codecov](https://codecov.io)

That job would run alongside, not replace, the existing `contracts` job
(WASM lint/check/test).

---

## WASM Compatibility Note

Soroban contracts compile to `wasm32-unknown-unknown` for on-chain deployment. `cargo-tarpaulin` is **incompatible** with WASM targets.

This implementation avoids the conflict by:
- Running coverage on the **native host** (`x86_64-unknown-linux-gnu`)
- Soroban's `testutils` feature enables a host-native simulation of the Soroban runtime, so all unit tests execute natively
- The `contracts` CI job (WASM linting/check) remains completely unchanged
