# Stellar Grants Contract Events

## Exportable Grant Receipts

Issue #135 adds two receipt-oriented contract events that can be indexed by off-chain tools:

- `PayerReceipt`
- `PayeeReceipt`

Both events include metadata fields designed for accounting exports:

- `grant_id`
- `recipient`
- `token`
- `amount`
- `milestone_index` (`None` for grant-level receipts)

`PayerReceipt` also includes:

- `memo` (optional memo passed by funder during `grant_fund`)

## When Receipts Are Emitted

- `PayerReceipt`: emitted on `grant_fund`.
- `PayeeReceipt`: emitted on `milestone_payout`, and on grant completion as a final summary snapshot.

## Querying Receipts

You can query contract events from Soroban RPC and filter by event type:

1. Fetch contract events for the deployed contract address.
2. Filter for `payer_receipt` and `payee_receipt` event names.
3. Parse payload fields (`grant_id`, `recipient`, `token`, `amount`, `milestone_index`, `memo`) for export.

Because these are structured contract events, indexers can store them directly in tabular format for CSV or accounting pipeline exports.
