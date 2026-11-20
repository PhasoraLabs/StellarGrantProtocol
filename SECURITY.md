# Security Policy

## Supported Versions

Only the `main` branch receives active security updates. We do not backport security fixes to earlier tags.

| Branch / Tag | Supported |
|--------------|-----------|
| `main` | Yes |
| Previous release tags | No |


## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

If you discover a security issue — in the smart contract, the frontend, the API, or the client SDK — please report it privately:

1. **GitHub Security Advisories (preferred):** [Open a private advisory](https://github.com/StellarGrant/stellargrant-fe/security/advisories/new). This creates an encrypted channel visible only to maintainers.
2. **Email:** Contact the project maintainers directly. See the GitHub organization profile for contact details.

Include as much detail as possible:
- A description of the vulnerability and its potential impact
- Steps to reproduce (or a proof-of-concept)
- Affected package(s) and versions
- Any suggested mitigation

You will receive an acknowledgment within 72 hours and a full response within 7 days. Critical vulnerabilities are patched on an emergency schedule.

We ask that you keep the vulnerability confidential until a fix has been released.


## Scope

### In-Scope

- **Smart contract vulnerabilities** — reentrancy, integer overflow/underflow, incorrect access control, escrow bypass, unauthorized payout
- **Frontend vulnerabilities** — XSS (especially through IPFS content rendering or user-supplied markdown), exposed secrets, transaction manipulation, wallet session hijacking
- **API vulnerabilities** — SQL injection, authentication bypass, information disclosure, SSRF
- **Dependency vulnerabilities** — high or critical CVEs in direct or transitive dependencies that are exploitable in this application's threat model
- **Logic bugs with financial impact** — anything that could cause loss of funds locked in escrow

### Out-of-Scope

- Issues in forked or unofficial deployments
- Vulnerabilities requiring physical access to the user's device
- Social engineering attacks
- Bugs in Stellar infrastructure itself (report those to [SDF](https://stellar.org/bug-bounty))
- Low-severity information disclosures with no practical attack path


## Automated Security Measures

### Dependency Scanning

- **`npm audit`** runs in CI on every PR targeting `web/` and `backend/`. The build fails if any `high` or `critical` severity vulnerabilities are found.
- **Dependabot** is configured to check for dependency updates weekly across npm and Cargo ecosystems. Dependabot PRs are reviewed and merged promptly.

### Static Analysis

- **ESLint** with security-focused rules runs on every PR for the frontend and API packages.
- **`cargo clippy`** with `-D warnings` runs on every PR for the contracts package, catching common Rust security anti-patterns.

### Content Security Policy

The frontend sets strict CSP headers via `next.config.ts`:
- `default-src 'self'` — no third-party scripts by default
- `connect-src` — whitelisted to Stellar RPC, Horizon, and Pinata only
- `frame-ancestors 'none'` — prevents clickjacking
- `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin`

---

## Patching Process

When a vulnerability is reported or detected:

1. **Triage** — Maintainers assess severity (Critical / High / Medium / Low) and attack feasibility within 72 hours.
2. **Fix** — A private branch is prepared with the fix. For contract vulnerabilities, the impact on deployed escrow funds is assessed and an emergency contract upgrade or migration plan is prepared if necessary.
3. **Test** — All relevant tests are run. For contract changes, a full audit of the affected functions is performed.
4. **Release** — The fix is merged to `main` and a tagged release is cut.
5. **Disclosure** — A GitHub Security Advisory is published with CVE if applicable. The disclosure includes a description of the vulnerability, impact, affected versions, and mitigation steps.

### Severity Definitions

| Severity | Description | Response Target |
|----------|-------------|-----------------|
| Critical | Funds at risk; active exploitation possible | Emergency patch within 24 hours |
| High | Significant data exposure or privilege escalation | Patch within 7 days |
| Medium | Limited impact, no direct fund loss risk | Patch within 30 days |
| Low | Minimal real-world impact | Addressed in next regular release |

---

## Security Best Practices for Contributors

- **Never commit secrets** — API keys, private keys, seeds, or passwords must never appear in source code or commit history. Use `.env.local` (gitignored).
- **Never use `NEXT_PUBLIC_` for secrets** — variables with this prefix are bundled into client-side JavaScript and visible to anyone.
- **Check arithmetic in contract code** — use `checked_add`, `checked_mul`, and `checked_sub` for all numeric operations in Rust to prevent overflow.
- **Validate all inputs at system boundaries** — any data arriving from user input, external APIs, or IPFS content must be validated and sanitized before use.
- **Use `DOMPurify` for rendered HTML** — the codebase already does this for user-supplied markdown; maintain this for any new HTML rendering.
- **Simulate before sending** — every contract write transaction is simulated first to verify resource fees and catch authorization failures before the user signs.
- **Follow the principle of least privilege** — components should only have access to the wallet or state they actually need.

---

## Known Security Considerations

### IPFS Content

Milestone proofs are fetched from IPFS via user-provided CIDs. The `ProofViewer` component sanitizes all rendered content through `DOMPurify`. Never render IPFS content as unsanitized HTML.

### Wallet Adapter Trust Boundary

The wallet adapter pattern means the application trusts the wallet extension to faithfully report the connected address and sign only the transaction XDR it receives. Users should only install wallets from official sources.

### Testnet vs. Mainnet

The `NEXT_PUBLIC_STELLAR_NETWORK` variable controls which network the app targets. The app validates that the connected wallet's network matches the configured network and shows a prominent warning on mismatch. Never deploy a production frontend pointing at mainnet using a testnet contract ID.
