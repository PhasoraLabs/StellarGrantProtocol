# Code Coverage

This document describes the automated code coverage setup for the `stellargrant-contracts` workspace.

---

## Overview

Coverage is measured using [`cargo-tarpaulin`](https://github.com/xd009642/tarpaulin) with the LLVM engine, targeting production contract logic only. Test files are excluded from all measurements.

- **Engine**: LLVM (`llvm-tools-preview`)
- **Scope**: `lib` targets across the workspace only
- **Excluded**: `*test.rs`, `*tests.rs`, `tests/*`
- **Output**: `cobertura.xml` (Cobertura/Codecov format)

---

## Configuration

**`.tarpaulin.toml`** (located in `stellargrant-contracts/`):

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
cd stellargrant-contracts
cargo tarpaulin --workspace --lib --target x86_64-unknown-linux-gnu --engine llvm --out Xml
```

> **Note**: This is the exact same command used in CI. The `.tarpaulin.toml` config is auto-detected.

### What is measured

| File | Included |
|---|---|
| `contracts/stellar-grants/src/lib.rs` | ✅ Yes |
| `contracts/stellar-grants/src/storage.rs` | ✅ Yes |
| `contracts/stellar-grants/src/types.rs` | ✅ Yes |
| `contracts/stellar-grants/src/events.rs` | ✅ Yes |
| `contracts/stellar-grants/src/test.rs` | ❌ Excluded |

---

## CI Integration

Coverage runs automatically on:
- Every **pull request**
- Every **push to `main`**

The coverage job in `.github/workflows/ci.yml`:

1. Sets up Rust with `llvm-tools-preview` component
2. Caches dependencies with `Swatinem/rust-cache`
3. Installs `cargo-tarpaulin`
4. Runs coverage on the **native host target** (`x86_64-unknown-linux-gnu`) — not WASM
5. Uploads `cobertura.xml` as a GitHub Actions artifact (retained for 7 days)
6. Uploads the report to [Codecov](https://codecov.io) with the `unittests` flag

> ⚠️ The `contracts` job (WASM build) is completely separate and **not affected** by the coverage pipeline.

---

## Codecov Setup

### Required Secret

Add the following secret to your GitHub repository:

**Settings → Secrets and variables → Actions → New repository secret**

| Name | Value |
|---|---|
| `CODECOV_TOKEN` | *(Token from your Codecov dashboard — Settings → Repository Upload Token)* |

<<<<<<< HEAD
> **Fork PR note**: GitHub does not expose repository secrets to workflows triggered from forks (security policy). This means coverage upload will silently skip on fork PRs but **will succeed** on pushes to `main` from the base repository. CI will still pass — `fail_ci_if_error` is set to `false` to handle this gracefully.

=======
>>>>>>> 2fd7037 (feat: add milestone deadlines, expiry logic, and fund reclaim mechanism)
### Badge

Add this to the root `README.md`, replacing `<owner>` and `<repo>` with your GitHub username and repository name:

```markdown
[![codecov](https://codecov.io/gh/<owner>/<repo>/branch/main/graph/badge.svg)](https://codecov.io/gh/<owner>/<repo>)
```

---

## WASM Compatibility Note

Soroban contracts compile to `wasm32-unknown-unknown` for on-chain deployment. `cargo-tarpaulin` is **incompatible** with WASM targets.

This implementation avoids the conflict by:
- Running coverage on the **native host** (`x86_64-unknown-linux-gnu`)
- Soroban's `testutils` feature enables a host-native simulation of the Soroban runtime, so all unit tests execute natively
- The `contracts` CI job (WASM linting/check) remains completely unchanged
