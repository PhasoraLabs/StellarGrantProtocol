# 🌌 Interactive Tutorial: StellarGrants For Beginners

Welcome new contributors! This step-by-step interactive tutorial is designed to get you comfortable with the StellarGrants Protocol on the Stellar blockchain. By the end of this guide, you will have stood up your own local testnet environment, deployed the smart contract, and routed a real grant from creation to milestone payout.

Let's dive in!

---


## 🛠 Prerequisites Checklist

Ensure your environment is set up with the basic tools:

- [x] **Rust (`>= 1.78`)**: Installed via `rustup`.
- [x] **WASM Target**: `rustup target add wasm32-unknown-unknown`
- [x] **Stellar CLI**: Installed via `cargo install --locked stellar-cli --features opt`
- [x] **Git**: Standard version control.

*Note: For detailed installation instructions, please refer to the main repository `README.md`.*

---

## 🎭 Step 1: Create and Fund Dummy Accounts

To simulate a real decentralized environment, we need three distinct participants:
1. **Alice (Creator)**: Creates the initial grant.
2. **Bob (Funder)**: Funds the grant with tokens.
3. **Charlie (Reviewer)**: Reviews the milestone submissions.

### 1a. Generate Identities

Run the following commands in your terminal to create these local identities:

```bash
stellar keys generate alice --network testnet
stellar keys generate bob --network testnet
stellar keys generate charlie --network testnet
```

### 1b. Fund with Friendbot

We will fund these newly created accounts automatically via Stellar's Testnet Friendbot. Since `--network testnet` was specified, `stellar-cli` will handle the funding process internally when using `stellar keys fund`, or you can manually fund using:

```bash
stellar keys fund alice --network testnet
stellar keys fund bob --network testnet
stellar keys fund charlie --network testnet
```

> ⚠️ **Warning:** Sometimes Friendbot can take a few seconds or experience rate limits. If funding fails, just wait a minute and retry!

You can check balances anytime using:
```bash
stellar keys balance alice --network testnet
```

---

## 🏗 Step 2: Compile the Smart Contract

Navigate natively to the contract's directory and compile it into a WebAssembly standard binary (`.wasm`).

```bash
cd contracts/contracts/stellar-grants
make build
# or explicitly: cargo build --target wasm32-unknown-unknown --release
```

Ensure the build output finished successfully. Your compiled contract will be located at:
`target/wasm32v1-none/release/stellar_grants.wasm`

---

## 🚀 Step 3: Deploy the Contract to Testnet

Alice will act as our protocol owner to deploy the contract. Execute the deploy command inside `contracts/stellar-grants`:

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/stellar_grants.wasm \
  --network testnet \
  --source-account alice \
  --alias stellar_grants
```

> **Note:** We used `--alias stellar_grants`. This saves the newly deployed contract address in your local config, allowing you to refer to it as `stellar_grants` in subsequent steps instead of pasting a long 56-character `C...` string!

### Initialize

Some Soroban contracts require initialization immediately after deployment. If applicable:
```bash
stellar contract invoke \
  --id stellar_grants \
  --network testnet \
  --source-account alice \
  -- initialize
```

---

## 💸 Step 4: Route a Full Grant

Now for the exciting part—let's create, fund, and payout a grant!

### 4a. Create a Grant (Alice)

Alice creates an open-source grant for a developer. We define 1 milestone and stipulate the reward.

```bash
stellar contract invoke \
  --id stellar_grants \
  --network testnet \
  --source-account alice \
  -- grant_create \
  --owner $(stellar keys address alice) \
  --title "Interactive Tutorial Bounties" \
  --description "A test task" \
  --total_amount 500 \
  --per_milestone 500 \
  --milestones 1
```

> **Tip:** Ensure this outputs your unique `grant_id`. Keep this number handy! In the examples below, we assume it's `1`.

### 4b. Fund the Grant (Bob)

Bob the philanthropist wants to sponsor Alice's cause. Before Bob can deposit funds into the contract's escrow, he must approve the token transfer from his account.

*(Assuming you're using Native XLM token, or a deployed token contract ID)*:

```bash
# Example syntax: Approve token transfer (Bob allows stellar_grants to spend 500)
# (Your exact token contract invoke may differ depending on the token standard integrated)

# 1. Fund the grant
stellar contract invoke \
  --id stellar_grants \
  --network testnet \
  --source-account bob \
  -- grant_fund \
  --grant_id 1 \
  --funder $(stellar keys address bob) \
  --amount 500
```

### 4c. Submit a Milestone (Grant Recipient)

The developer has finished their work! They submit proof to the contract. (Assuming Alice is the dev, or she delegates it):

```bash
stellar contract invoke \
  --id stellar_grants \
  --network testnet \
  --source-account alice \
  -- milestone_submit \
  --grant_id 1 \
  --milestone_index 0 \
  --notes "Completed Interactive Tutorial creation!" \
  --proof_url "https://github.com/StellarGrant/StellarGrant-Contracts"
```

### 4d. Vote and Payout (Charlie)

Reviewer Charlie validates the submission. When he votes to approve, the smart contract automatically triggers the milestone payment!

```bash
stellar contract invoke \
  --id stellar_grants \
  --network testnet \
  --source-account charlie \
  -- milestone_vote \
  --grant_id 1 \
  --milestone_index 0 \
  --reviewer $(stellar keys address charlie) \
  --approve true
```

*Boom!* 💥 You should see your transaction finalize. Once the required quorum is reached (e.g., 1 reviewer in this dummy test), the 500 tokens were unescrowed and delivered.

---

## 🎉 Conclusion

You successfully implemented a fully decentralized routing process for a grant on the Stellar network.  

**Next Steps**:
- Read the [Contribution Guide](./contracts/ContributionGuide.md)
- Pick up an open issue from the `issues/` directory.
- Check out [Soroban documentation](https://developers.stellar.org/docs/build/smart-contracts/overview) for deeper insights.

*Happy Coding on Stellar!* 🌊
