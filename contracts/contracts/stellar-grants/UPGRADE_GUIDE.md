# Stellar Grants contract — upgrade process

## Roles

- **Global admin** (set in `initialize`, rotated via `admin_change`): sole address allowed to call `admin_upgrade`, `set_council`, `set_staking_config`, `set_identity_oracle`, and `slash_reviewer` (in addition to existing auth rules on other functions).

## Storage version

- Persistent key `StorageVersion` (`u32`) tracks upgrade generations. It is set to `1` on first `initialize` and incremented immediately before each successful `admin_upgrade`.
- After deploying new WASM, read `get_contract_storage_version` off-chain or in clients to detect when migration logic is required.

## Upgrading WASM

1. Build the new contract: `cargo build --target wasm32v1-none --release` (or the workspace’s documented profile).
2. Compute the WASM file hash (32 bytes) as required by your tooling; Stellar CLI can help publish the WASM and obtain the hash.
3. Invoke `admin_upgrade` with the global admin account and `new_wasm_hash: BytesN<32>`.

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --network testnet \
  --source-account YOUR_ADMIN_SECRET \
  -- \
  admin_upgrade \
  --admin ADMIN_ADDRESS \
  --new_wasm_hash <32-byte-hex>
```

4. The contract emits `ContractWasmUpgraded` with the new hash and new storage version. Indexers should treat this as a signal to revalidate assumptions.

## Safety

- `admin_upgrade` must be the last successful mutation in a transaction that changes code; the host replaces the WASM in place.
- Always test upgrades on Futurenet/Testnet with a snapshot of production storage layout.
- If a future version needs data migration, gate reads/writes on `StorageVersion` inside the new WASM and document the migration in this file.
