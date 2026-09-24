# Code Coverage

This document describes the code coverage setup and local measurement workflows for the `contracts` workspace.

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

> **Note**: This is the exact same command used in CI. The `.tarpaulin.toml` config is auto-detected.

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

## CI Integration Status

> ℹ️ **Current Status: Local-Only Workflow**  
> Code coverage is currently a **local-only workflow** (`cargo-tarpaulin` / `cargo llvm-cov`) using the configuration in `contracts/.tarpaulin.toml`. The active `.github/workflows/ci.yml` pipeline runs the `contracts` verification suite (formatting, clippy checks, and `cargo test`), but does not currently include an automated Tarpaulin coverage generation or Codecov upload job.

### Planned CI Pipeline Workflow

When automated coverage is enabled in `.github/workflows/ci.yml`, the scheduled pipeline will:

1. Set up Rust with the `llvm-tools-preview` component
2. Cache dependencies with `Swatinem/rust-cache`
3. Install `cargo-tarpaulin`
4. Run coverage on the native host target (`x86_64-unknown-linux-gnu`) using `contracts/.tarpaulin.toml`
5. Upload `cobertura.xml` as a GitHub Actions artifact
6. Upload the report to [Codecov](https://codecov.io) with the `unittests` flag

---

## Codecov Setup

### Required Secret

Add the following secret to your GitHub repository:

**Settings → Secrets and variables → Actions → New repository secret**

| Name | Value |
|---|---|
| `CODECOV_TOKEN` | *(Token from your Codecov dashboard — Settings → Repository Upload Token)* |

> **Fork PR note**: GitHub does not expose repository secrets to workflows triggered from forks (security policy). This means coverage upload will silently skip on fork PRs but **will succeed** on pushes to `main` from the base repository. CI will still pass — `fail_ci_if_error` is set to `false` to handle this gracefully.

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
