# StellarGrants API Middleware

Backend middleware for caching grant state and validating signed writes.

## Endpoints

- `GET /health`
- `GET /grants`
- `GET /grants/:id`
- `POST /milestone_proof`

## Signature format for `POST /milestone_proof`

The server verifies Ed25519 signatures against the Stellar public key in `submittedBy`.

Message format:

`stellargrant:milestone_proof:v1|{grantId}|{milestoneIdx}|{proofCid}|{submittedBy}|{nonce}|{timestamp}`

Signature must be base64-encoded raw signature bytes.

## Run locally

```bash
cd backend
npm install
npm run dev
```

## E2E tests

```bash
cd backend
npm run test:e2e
```
