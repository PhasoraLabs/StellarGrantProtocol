# Contributing to StellarGrants Protocol

Thank you for your interest in contributing! StellarGrants is an active open-source project with over 60 contributors building milestone-based grant infrastructure on the Stellar blockchain. Every contribution — bug fix, new feature, documentation improvement, or test — moves the ecosystem forward.

This document is the **root-level contribution guide** covering all packages in this monorepo. For package-specific guidance see:
- [web/CONTRIBUTING.md](web/CONTRIBUTING.md) — frontend-specific guidelines, Wave Program, and available issues
- [contracts/ContributionGuide.md](contracts/ContributionGuide.md) — contract conventions and testing

---

## Table of Contents

- [Wave Program](#wave-program)
- [Code of Conduct](#code-of-conduct)
- [Ways to Contribute](#ways-to-contribute)
- [Before You Start](#before-you-start)
- [Fork & Local Setup](#fork--local-setup)
- [Branch & Commit Conventions](#branch--commit-conventions)
- [Coding Standards](#coding-standards)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting](#issue-reporting)
- [Review Expectations](#review-expectations)
- [Getting Help](#getting-help)
- [Recognition](#recognition)

---

## Wave Program

StellarGrants participates in the **Stellar Wave Program** on [Drips](https://drips.network/wave/stellar). Issues labeled `drips-wave` are eligible for Wave Point rewards.

- Browse eligible issues: filter by `drips-wave` label on [GitHub Issues](https://github.com/StellarGrant/stellargrant-fe/issues?q=is%3Aopen+label%3Adrips-wave)
- Claim an issue by commenting on it before you start coding
- Open a draft PR early so maintainers can give feedback before you finish
- Include before/after screenshots for all visible UI changes — they earn faster reviews

---

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating you agree to uphold it. Please report unacceptable behavior to the project maintainers via GitHub or [opening a private security advisory](https://github.com/StellarGrant/stellargrant-fe/security/advisories/new).

---

## Ways to Contribute

| Type | Examples |
|------|---------|
| **Bug fixes** | Fix a broken UI state, a failed contract call, a failing test |
| **Features** | Implement a new page, hook, contract function, or SDK method |
| **Tests** | Add Vitest unit tests, Playwright E2E tests, or Rust contract tests |
| **Documentation** | Improve READMEs, add JSDoc, write tutorials, update ARCHITECTURE.md |
| **Performance** | Bundle size, RPC call batching, rendering optimizations |
| **Accessibility** | ARIA labels, keyboard navigation, color contrast |
| **DevEx** | CI improvements, Storybook stories, Makefile targets |

Not sure where to start? Look for issues labeled [`good first issue`](https://github.com/StellarGrant/stellargrant-fe/issues?q=is%3Aopen+label%3A%22good+first+issue%22) or read the [beginner tutorial](TUTORIAL.md).

---

## Before You Start

1. **Search existing issues and PRs** — your idea may already be in progress
2. **Open an issue first** for non-trivial features or architectural changes — aligning before coding saves everyone time
3. **Check the project board** for in-progress work
4. **Read the relevant package README** — each package has its own conventions

---

## Fork & Local Setup

### 1. Fork and Clone

```bash
# Fork on GitHub, then:
git clone https://github.com/YOUR_USERNAME/stellargrant-fe.git
cd stellargrant-fe
git remote add upstream https://github.com/StellarGrant/stellargrant-fe.git
```

### 2. Install Dependencies per Package

Each package manages its own dependencies independently.

**Frontend:**
```bash
cd web
npm ci
cp .env.local.example .env.local
# Fill in NEXT_PUBLIC_CONTRACT_ID and other required values
```

**Contracts:**
```bash
cd contracts
rustup target add wasm32-unknown-unknown
cargo check --workspace --target wasm32-unknown-unknown
```

**Client SDK:**
```bash
cd client
npm ci
npm run build
```

**API (optional):**
```bash
cd backend
npm ci
# Requires PostgreSQL — see docker-compose.yml
```

### 3. Verify Setup

```bash
# Frontend: dev server should start with no errors
cd web && npm run dev

# Frontend: lint and tests should pass
npm run lint && npm run test:run && npm run build

# Contracts: clippy should be clean
cd ../contracts
cargo clippy --workspace --lib --target wasm32-unknown-unknown -- -D warnings
cargo test
```

### 4. Keep Your Fork Up to Date

```bash
git fetch upstream
git checkout main
git merge upstream/main
git push origin main
```

Rebase your feature branch before opening a PR:

```bash
git checkout your-feature-branch
git rebase upstream/main
```

---

## Branch & Commit Conventions

### Branch Naming

```
<type>/<issue-ref>-<short-description>

feat/FE-01-wallet-connect-modal
fix/FE-23-funding-progress-calculation
docs/root-readme-overhaul
refactor/stellar-client-singleton
test/milestone-vote-hook
```

Branch types: `feat`, `fix`, `docs`, `refactor`, `test`, `style`, `chore`

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

[optional body]

[optional footer: Closes #N]
```

**Examples:**

```
feat(wallet): add xBull adapter with connection + sign methods

Adds AlbedoAdapter and xBullAdapter following the WalletAdapter interface
defined in lib/wallets/types.ts. Updates WalletSelectModal to list both options.

Closes #FE-01
```

```
fix(grants): correct XLM/stroops conversion in FundingProgress

The progress bar was reading raw stroops instead of XLM, making 100 XLM
look like 0.00001% funded. Divide raw amount by 10_000_000 before display.

Fixes #112
```

```
test(hooks): add Vitest unit tests for useFundGrant

Covers happy path, insufficient balance error, and network timeout cases.
Mocks the ContractClient using vi.mock.
```

**Commit types:** `feat` · `fix` · `docs` · `style` · `refactor` · `test` · `chore` · `perf` · `ci`

---

## Coding Standards

### TypeScript (Frontend & SDK)

- **No `any`** — use `unknown` with type narrowing if the type is genuinely unknown
- **Strict mode** — `tsconfig.json` enables all strict checks; they must pass
- **Explicit interfaces for all public APIs** — props, hook return values, API responses
- **Prefer `type` for unions/intersections, `interface` for object shapes**

```typescript
// Good
interface GrantCardProps {
  grant: Grant;
  onClick?: (id: string) => void;
}

// Bad
function GrantCard(props: any) { ... }
```

### React & Next.js

- **Server Components by default** — add `"use client"` only when browser APIs or interactivity require it
- **Custom hooks for reusable logic** — keep component files focused on rendering
- **No prop drilling past two levels** — use Zustand or TanStack Query
- **Error boundaries around wallet interactions** — wallet calls can fail; handle gracefully

```typescript
// Good — Server Component default
export default async function GrantPage({ params }: { params: { id: string } }) {
  const grant = await fetchGrant(params.id);
  return <GrantDetail grant={grant} />;
}

// Good — Client Component only where needed
"use client";
export function VotePanel({ milestoneIdx }: { milestoneIdx: number }) {
  const { vote, isPending } = useVoting(milestoneIdx);
  ...
}
```

### Styling

- **Tailwind CSS only** — no inline styles, no CSS modules, no styled-components
- **shadcn/ui for base components** — extend them, never edit the source files in `components/ui/`
- **Mobile-first** — write base styles for small screens, override for `md:` and `lg:`
- **Use design tokens** from `tailwind.config.ts` rather than arbitrary values

### Rust (Contracts)

- Format with `cargo fmt` before every commit
- `cargo clippy -- -D warnings` must be clean
- Numeric operations must be checked for overflow (`checked_add`, `checked_mul`)
- All public functions must have corresponding unit tests in `#[cfg(test)]` blocks

### File & Symbol Naming

| Thing | Convention | Example |
|-------|-----------|---------|
| React components | PascalCase | `GrantCard.tsx` |
| Custom hooks | camelCase, `use` prefix | `useFundGrant.ts` |
| Utility functions | camelCase | `formatStroops.ts` |
| TypeScript types | PascalCase | `Grant`, `Milestone` |
| Constants | UPPER_SNAKE_CASE | `MAX_MILESTONES` |
| Pages (App Router) | `page.tsx` | `app/grants/page.tsx` |

---

## Pull Request Process

### Pre-Submission Checklist

Run these locally and confirm all pass before opening a PR:

```bash
cd web
npm run lint
npm run test:run
npm run build
```

For contract changes:
```bash
cd contracts
cargo fmt --all -- --check
cargo clippy --workspace --lib --target wasm32-unknown-unknown -- -D warnings
cargo test
```

### PR Description Template

```markdown
## What does this PR do?
<!-- One paragraph summary -->

## Related Issue
Closes #N

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Refactor
- [ ] Documentation
- [ ] Tests
- [ ] Breaking change

## Screenshots / Screen Recordings
<!-- Required for any visible UI change. Add before/after pairs. -->

## Testing Done
- [ ] Unit tests added or updated
- [ ] E2E tests added or updated (if applicable)
- [ ] Manually tested on testnet with Freighter wallet
- [ ] Tested on mobile viewport (for UI changes)

## Checklist
- [ ] `npm run lint` passes
- [ ] `npm run test:run` passes
- [ ] `npm run build` succeeds
- [ ] No new TypeScript errors
- [ ] No secrets or environment values committed
- [ ] Documentation updated (if applicable)
- [ ] Conventional Commit message format used
```

### What Happens After You Open a PR

1. **Automated CI** runs lint, build, and tests across all affected packages
2. **A maintainer** will review within a few days (Wave issues get priority)
3. **Address feedback** by pushing new commits to the same branch
4. **Squash and merge** — maintainers typically squash on merge; keep your history clean but don't squash manually unless asked

---

## Issue Reporting

### Before Filing

- Search open and closed issues for duplicates
- Verify you are on the latest `main`
- For contract bugs: confirm the transaction hash and network

### Bug Report

```markdown
**Describe the bug**
A clear description of what went wrong.

**To Reproduce**
1. Go to '...'
2. Click '...'
3. See error

**Expected behavior**
What you expected to happen.

**Screenshots / Console errors**
Paste any relevant browser console output or screenshots.

**Environment**
- OS:
- Browser + version:
- Node.js version:
- Wallet extension + version:
- Network (testnet / mainnet):
```

### Feature Request

```markdown
**Problem / motivation**
What user need or gap does this address?

**Proposed solution**
How would you like this to work? Include mockups or pseudocode if helpful.

**Alternatives considered**
Other approaches you evaluated.
```

---

## Review Expectations

- Reviewers focus on correctness, security, and maintainability — not style nits (the linter handles those)
- All review comments are suggestions unless marked `[BLOCKING]`
- If you disagree with feedback, explain your reasoning in the thread — healthy debate is welcome
- Reviews may take 1–5 business days depending on complexity and maintainer availability

---

## Getting Help

| Channel | Use for |
|---------|---------|
| [GitHub Issues](https://github.com/StellarGrant/stellargrant-fe/issues) | Bug reports, feature requests |
| [GitHub Discussions](https://github.com/StellarGrant/stellargrant-fe/discussions) | Questions, ideas, general discussion |
| Issue comments | Asking about a specific issue you're working on |
| PR comments | Implementation questions on in-flight work |

When asking for help, always include:
- What you were trying to do
- What you tried
- What error or unexpected result you got
- Relevant code snippet or PR link

---

## Recognition

- All contributors are listed in [GitHub's contributor graph](https://github.com/StellarGrant/stellargrant-fe/graphs/contributors)
- Significant contributions are called out in release notes
- Consistent high-quality contributors may be invited to the maintainer team
- Wave Program participants earn Wave Points for completed `drips-wave` issues

---

By contributing you agree that your work will be licensed under the project's [MIT License](LICENSE).

**Thank you for helping build StellarGrants.**
