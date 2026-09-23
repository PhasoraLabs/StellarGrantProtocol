Here's the merged file with the fix applied:

// File: .github/workflows/ci.yml
name: CI

on:
  push:
    branches:
      - main
  pull_request:
  release:
    types: [published]

jobs:
  contracts:
    name: Contracts (Rust)
    runs-on: ubuntu-latest
    env:
      SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2: "1"
    defaults:
      run:
        working-directory: stellargrant-contracts
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Add WASM target
        run: rustup target add wasm32v1-none

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy (WASM, lib only)
        run: cargo clippy --workspace --lib --target wasm32v1-none -- -D warnings

      - name: Check (WASM)
        run: cargo check --workspace --target wasm32v1-none

  api:
    name: API (Node.js)
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres: