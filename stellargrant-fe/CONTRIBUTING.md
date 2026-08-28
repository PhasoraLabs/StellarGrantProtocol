# Contributing to StellarGrants Frontend

Thank you for your interest in contributing to the StellarGrants Protocol frontend! This document provides guidelines and instructions for contributing to the project.

## 🌊 Wave Program

The StellarGrants frontend is part of the **Stellar Wave Program** on Drips. Contributors can earn Wave Points by completing issues labeled with `drips-wave`. All frontend issues are designed to be Wave-friendly, with clear acceptance criteria and direct mapping to UI features.

**Learn more**: [drips.network/wave/stellar](https://drips.network/wave/stellar)

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting](#issue-reporting)
- [Frontend-Specific Guidelines](#frontend-specific-guidelines)
- [Testing Requirements](#testing-requirements)
- [Available Issues](#available-issues)

## 📜 Code of Conduct

This project adheres to the Contributor Covenant Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

### Our Standards

- Be respectful and inclusive
- Welcome newcomers and help them learn
- Focus on constructive feedback
- Celebrate diverse perspectives

## 🚀 Getting Started

### Prerequisites

Before you begin, ensure you have:

- **Node.js** >= 18.17 installed
- **pnpm** >= 8 installed (`npm install -g pnpm`)
- **Git** configured with your credentials
- **Freighter Wallet** extension installed (for testing wallet features)
- A basic understanding of:
  - React and Next.js
  - TypeScript
  - Stellar blockchain concepts
  - Git and GitHub workflows

### Initial Setup

1. **Fork the repository**

   ```bash
   # Click "Fork" on GitHub, then clone your fork
   git clone https://github.com/YOUR_USERNAME/stellargrant-fe.git
   cd stellargrant-fe
   ```

2. **Add upstream remote**

   ```bash
   git remote add upstream https://github.com/your-org/stellargrant-fe.git
   ```

3. **Install frontend dependencies**

   ```bash
   cd stellargrant-fe
   pnpm install
   ```

4. **Set up environment variables**

   ```bash
   cp .env.example .env.local
   # Edit .env.local with your testnet contract ID and API keys
   ```

5. **(Optional) Set up the mock API server**

   The project includes a mock API server for development and testing. It provides endpoints for caching grant state and validating signed writes.

   ```bash
   cd api
   npm install
   npm run dev
   ```

   The mock server runs on port 4000 by default. You can configure this via the `PORT` environment variable.

6. **Start development server**

   ```bash
   cd stellargrant-fe
   pnpm dev
   ```

7. **Verify setup**

   - Open [http://localhost:3000](http://localhost:3000)
   - Check that the app loads without errors
   - Run `pnpm lint` and `pnpm type-check` to ensure everything passes
   - If using the mock server, verify it's running at [http://localhost:4000/health](http://localhost:4000/health)

## � Project Structure

Understanding the folder structure is essential for navigating the codebase and making contributions.

### Frontend Structure (stellargrant-fe/)

```
stellargrant-fe/
├── app/                          # Next.js App Router pages
│   ├── layout.tsx                # Root layout with fonts and providers
│   ├── page.tsx                  # Homepage / landing page
│   ├── grants/                   # Grant-related pages
│   │   ├── page.tsx              # Grant listing page
│   │   ├── [id]/
│   │   │   ├── page.tsx          # Grant detail view
│   │   │   ├── fund/page.tsx     # Fund grant flow
│   │   │   └── milestones/
│   │   │       ├── page.tsx      # Milestone list
│   │   │       └── [idx]/page.tsx # Milestone detail + vote
│   │   └── create/page.tsx       # Create grant form
│   ├── profile/page.tsx          # Contributor profile
│   ├── leaderboard/page.tsx      # Contributor reputation board
│   └── api/                      # API routes (if any)
├── components/
│   ├── ui/                       # shadcn/ui base components
│   ├── grants/                   # Grant-specific components
│   ├── milestones/               # Milestone-related components
│   ├── wallet/                   # Wallet connection components
│   └── layout/                   # Header, Footer, Sidebar
├── hooks/                        # Custom React hooks
│   ├── useWallet.ts              # Wallet connection and state
│   ├── useGrants.ts              # Grant data fetching
│   ├── useGrant.ts               # Single grant operations
│   ├── useMilestone.ts           # Milestone operations
│   ├── useContractTransaction.ts # Contract transaction handling
│   ├── useContractEvents.ts      # Event streaming
│   └── useIPFS.ts                # IPFS file upload
├── lib/
│   ├── stellar/                  # Stellar SDK wrappers
│   │   ├── client.ts             # RPC client singleton
│   │   ├── contract.ts           # Contract call helpers
│   │   └── events.ts             # Event streaming
│   ├── store/                    # Zustand global state
│   │   ├── walletStore.ts        # Wallet state management
│   │   └── index.ts              # Store exports
│   ├── utils/                    # Shared utility functions
│   │   └── index.ts              # Utility exports
│   └── config/                   # Configuration files
│       └── env-validation.ts     # Environment variable validation
├── types/                        # Global TypeScript types
│   └── index.ts                  # Type definitions
├── public/                       # Static assets
│   ├── next.svg
│   ├── vercel.svg
│   └── window.svg
├── tests/                        # Vitest unit tests
├── e2e/                          # Playwright end-to-end tests
├── .env.local                    # Environment variables (not committed)
├── .env.example                  # Environment variable template
├── next.config.ts                # Next.js configuration
├── tailwind.config.ts            # Tailwind CSS configuration
├── tsconfig.json                # TypeScript configuration
└── package.json                  # Dependencies and scripts
```

### API Structure (api/)

```
api/
├── src/
│   ├── index.ts                  # API entry point and bootstrap
│   ├── app.ts                    # Express app configuration
│   ├── config/
│   │   ├── env.ts                # Environment configuration
│   │   └── env-validation.ts     # Environment variable validation
│   ├── db/
│   │   └── data-source.ts        # TypeORM data source
│   ├── entities/                 # Database entities
│   │   ├── Grant.ts
│   │   ├── Milestone.ts
│   │   └── Contributor.ts
│   ├── middlewares/              # Express middleware
│   ├── routes/                   # API route handlers
│   │   ├── grants.ts
│   │   └── milestones.ts
│   ├── scripts/                  # Utility scripts
│   │   └── sync-db.ts            # Database synchronization
│   ├── services/                 # Business logic
│   │   ├── grantService.ts
│   │   └── milestoneService.ts
│   └── soroban/                  # Stellar contract integration
│       ├── mock-client.ts        # Mock Soroban contract client
│       └── contract-client.ts    # Real contract client
├── tests/
│   └── e2e/                      # End-to-end tests
├── Dockerfile                    # Docker configuration
├── tsconfig.json                # TypeScript configuration
├── vitest.config.ts             # Vitest configuration
└── package.json                  # Dependencies and scripts
```

### Key Directories Explained

- **app/**: Next.js App Router pages. Each file corresponds to a route. Dynamic routes use `[param]` syntax.
- **components/**: Reusable React components. Organized by feature (grants, milestones, wallet) and by type (ui, layout).
- **hooks/**: Custom React hooks that encapsulate reusable logic. Hooks are the primary way to share stateful logic.
- **lib/**: Core library code including Stellar SDK wrappers, state management, and utilities.
- **types/**: Shared TypeScript type definitions used across the application.
- **api/**: Backend API server (optional for development). Provides caching and validation endpoints.

### File Naming Conventions

- **Components**: PascalCase (e.g., `GrantCard.tsx`, `WalletConnect.tsx`)
- **Hooks**: camelCase with `use` prefix (e.g., `useWallet.ts`, `useGrants.ts`)
- **Utilities**: camelCase (e.g., `formatAddress.ts`, `validateForm.ts`)
- **Types**: PascalCase (e.g., `Grant.ts`, `Milestone.ts`)
- **Pages**: `page.tsx` (Next.js App Router convention)
- **Layouts**: `layout.tsx` (Next.js App Router convention)

## �🔄 Development Workflow

### Branch Naming

Use descriptive branch names that reference the issue number:

```bash
# Format: type/issue-number-short-description
git checkout -b feat/FE-01-wallet-connect-modal
git checkout -b fix/FE-12-wallet-hook-tests
git checkout -b docs/FE-09-ci-setup
```

**Branch Types:**
- `feat/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions/updates
- `style/` - Code style changes (formatting, etc.)

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Examples:**

```
feat(wallet): implement Freighter connection modal

Add WalletConnect component with support for Freighter, xBull, and Passkey wallets.
Includes wallet detection, connection flow, and error handling.

Closes FE-01
```

```
fix(grants): correct funding progress calculation

Fix bug where funding progress bar showed incorrect percentage when multiple tokens were deposited.

Fixes #123
```

**Commit Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, semicolons, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### Keeping Your Fork Updated

Regularly sync your fork with the upstream repository:

```bash
# Fetch latest changes from upstream
git fetch upstream

# Switch to main branch
git checkout main

# Merge upstream changes
git merge upstream/main

# Push to your fork
git push origin main
```

## 📝 Coding Standards

### TypeScript

- **Always use TypeScript** - No `any` types unless absolutely necessary
- **Define interfaces** for all props, state, and API responses
- **Use type inference** where possible, but be explicit for public APIs
- **Enable strict mode** - All TypeScript strict checks must pass

```typescript
// ✅ Good
interface GrantCardProps {
  grant: Grant;
  onClick?: () => void;
  showOwner?: boolean;
}

export function GrantCard({ grant, onClick, showOwner = false }: GrantCardProps) {
  // ...
}

// ❌ Bad
export function GrantCard(props: any) {
  // ...
}
```

### React & Next.js

- **Use functional components** with hooks
- **Server Components by default** - Only use `"use client"` when necessary
- **Follow Next.js App Router conventions**
- **Use TypeScript for all components**

```typescript
// ✅ Good - Server Component (default)
export default async function GrantPage({ params }: { params: { id: string } }) {
  const grant = await fetchGrant(params.id);
  return <GrantDetail grant={grant} />;
}

// ✅ Good - Client Component (when needed)
"use client";

export function WalletConnect() {
  const { address, connect } = useWallet();
  // ...
}
```

### Component Structure

1. **Imports** (grouped and sorted)
2. **Types/Interfaces**
3. **Component**
4. **Exports**

```typescript
// 1. External imports
import { useState } from "react";
import { useWallet } from "@/hooks/useWallet";

// 2. Internal imports
import { Button } from "@/components/ui/button";
import { GrantCard } from "@/components/grants/GrantCard";

// 3. Types
interface GrantListProps {
  grants: Grant[];
  onGrantClick?: (grant: Grant) => void;
}

// 4. Component
export function GrantList({ grants, onGrantClick }: GrantListProps) {
  // Component logic
}

// 5. Exports (if needed)
export type { GrantListProps };
```

### Styling

- **Use Tailwind CSS** for all styling
- **Follow design tokens** from `tailwind.config.ts`
- **Use shadcn/ui components** as base primitives
- **Mobile-first** responsive design

```typescript
// ✅ Good
<div className="flex flex-col gap-4 p-6 bg-stellar-navy text-white rounded-lg">
  <h2 className="text-2xl font-bold">Grant Title</h2>
  <p className="text-muted-foreground">Grant description</p>
</div>

// ❌ Bad
<div style={{ padding: "24px", backgroundColor: "#0F2444" }}>
  {/* Inline styles */}
</div>
```

### File Naming

- **Components**: PascalCase (e.g., `GrantCard.tsx`)
- **Hooks**: camelCase with `use` prefix (e.g., `useWallet.ts`)
- **Utilities**: camelCase (e.g., `formatAddress.ts`)
- **Types**: PascalCase (e.g., `Grant.ts`)
- **Constants**: UPPER_SNAKE_CASE (e.g., `CONTRACT_ADDRESSES.ts`)

### Import Organization

```typescript
// 1. React and Next.js
import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";

// 2. External libraries
import { StellarSdk } from "@stellar/stellar-sdk";
import { useQuery } from "@tanstack/react-query";

// 3. Internal absolute imports (@/)
import { useWallet } from "@/hooks/useWallet";
import { Button } from "@/components/ui/button";

// 4. Relative imports
import { GrantCard } from "./GrantCard";
import { formatDate } from "../utils/date";
```

## 🔍 Pull Request Process

### Before Submitting

1. **Update your branch** with latest changes from `main`
2. **Run all checks locally**:
   ```bash
   pnpm lint
   pnpm type-check
   pnpm test
   pnpm build
   ```
3. **Write or update tests** for your changes
4. **Update documentation** if needed
5. **Add screenshots** for UI changes (especially for Wave issues)

### PR Checklist

- [ ] Code follows the project's coding standards
- [ ] All tests pass (`pnpm test`)
- [ ] TypeScript compiles without errors (`pnpm type-check`)
- [ ] Linting passes (`pnpm lint`)
- [ ] Build succeeds (`pnpm build`)
- [ ] Documentation updated (if applicable)
- [ ] Screenshots added for UI changes
- [ ] Issue number referenced in PR description
- [ ] Commit messages follow Conventional Commits

### PR Description Template

```markdown
## Description
Brief description of changes

## Related Issue
Closes #FE-XX

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Screenshots (if applicable)
<!-- Add screenshots for UI changes -->

## Testing
- [ ] Unit tests added/updated
- [ ] E2E tests added/updated (if applicable)
- [ ] Tested on testnet
- [ ] Tested with Freighter wallet

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
- [ ] No new warnings generated
```

### Review Process

1. **Automated checks** must pass (CI/CD)
2. **At least one maintainer** must approve
3. **Address review feedback** promptly
4. **Squash commits** if requested (maintainers will handle this)

## 🐛 Issue Reporting

### Before Creating an Issue

1. **Search existing issues** to avoid duplicates
2. **Check if it's already fixed** in the latest version
3. **Verify it's a frontend issue** (not a contract issue)

### Bug Report Template

```markdown
## Bug Description
Clear and concise description of the bug.

## Steps to Reproduce
1. Go to '...'
2. Click on '...'
3. Scroll down to '...'
4. See error

## Expected Behavior
What you expected to happen.

## Actual Behavior
What actually happened.

## Screenshots
If applicable, add screenshots.

## Environment
- OS: [e.g., macOS 14.0]
- Browser: [e.g., Chrome 120]
- Node.js version: [e.g., 20.10.0]
- Wallet: [e.g., Freighter 2.0.0]
- Network: [e.g., testnet]

## Additional Context
Any other relevant information.
```

### Feature Request Template

```markdown
## Feature Description
Clear and concise description of the feature.

## Use Case
Why is this feature needed? What problem does it solve?

## Proposed Solution
How would you like this feature to work?

## Alternatives Considered
Other solutions you've considered.

## Additional Context
Screenshots, mockups, or examples.
```

## 🎨 Frontend-Specific Guidelines

### Component Development

1. **Start with Server Components** - Only use Client Components when needed
2. **Use custom hooks** for reusable logic
3. **Keep components small** - Single responsibility principle
4. **Compose, don't modify** - Extend shadcn/ui components, don't edit them directly

### Wallet Integration

- **Always check wallet connection** before wallet operations
- **Handle errors gracefully** - Show user-friendly error messages
- **Support multiple wallets** - Don't hardcode Freighter-only flows
- **Test with testnet** - Never use mainnet for development

```typescript
// ✅ Good
const { address, isConnected, connect } = useWallet();

if (!isConnected) {
  return <WalletConnectPrompt />;
}

// ❌ Bad
const address = await getAddress(); // May throw if not connected
```

### Contract Interactions

- **Use ContractClient** - Don't call RPC directly
- **Simulate before signing** - Always simulate transactions first
- **Handle resource fees** - Include proper resource estimates
- **Poll for status** - Don't assume immediate success

```typescript
// ✅ Good
const { execute, isPending } = useContractTransaction();

await execute({
  method: "grantFund",
  args: { grant_id: grantId, token, amount },
  onSuccess: () => toast.success("Grant funded!"),
  onError: (error) => toast.error(error.message),
});

// ❌ Bad
const tx = await contract.grantFund(...);
await signAndSubmit(tx); // No error handling
```

### State Management

- **Zustand for global state** - Wallet, user preferences
- **TanStack Query for server state** - Grants, milestones, contract data
- **Local state for UI** - Form inputs, modal open/close
- **Avoid prop drilling** - Use context or Zustand when needed

### Performance

- **Use React.memo** for expensive components
- **Lazy load** heavy components
- **Optimize images** with Next.js Image component
- **Debounce search inputs**
- **Virtualize long lists**

## ✅ Testing Requirements

### Unit Tests

- **Test all custom hooks** - Use React Testing Library
- **Test utility functions** - Pure functions should have 100% coverage
- **Mock contract calls** - Don't make real RPC calls in tests

```typescript
// Example: tests/hooks/useWallet.test.ts
import { renderHook, waitFor } from "@testing-library/react";
import { useWallet } from "@/hooks/useWallet";

describe("useWallet", () => {
  it("connects to Freighter wallet", async () => {
    const { result } = renderHook(() => useWallet());
    await result.current.connect("freighter");
    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });
  });
});
```

### Component Tests

- **Test user interactions** - Clicks, form submissions
- **Test conditional rendering** - Different states
- **Test accessibility** - ARIA labels, keyboard navigation

### E2E Tests

- **Critical user flows** - Grant creation, funding, milestone submission
- **Wallet integration** - Connection, signing transactions
- **Cross-browser testing** - Chrome, Firefox, Safari

### Coverage Goals

- **Minimum 80%** code coverage for new code
- **100% coverage** for utility functions
- **Critical paths** should have E2E tests

## 📋 Available Issues

Frontend issues are labeled with `drips-wave` and prefixed with `FE-XX`. Here are some examples:

### Good First Issues

- **FE-02**: Build GrantCard component with status badge and funding bar
- **FE-04**: Implement FundingProgress animated bar component
- **FE-09**: Set up GitHub Actions CI for frontend (lint, test, build)
- **FE-10**: Add dark mode support using next-themes
- **FE-12**: Write Vitest tests for useWallet hook

### Medium Difficulty

- **FE-01**: Implement WalletConnect modal with Freighter + xBull + Passkey tabs
- **FE-05**: Build VotePanel with quorum progress and reviewer list
- **FE-07**: Implement IPFS file upload hook and ProofViewer component
- **FE-11**: Implement transaction status polling with animated feedback
- **FE-13**: Add WalletGuard role-based access wrapper component
- **FE-14**: Build leaderboard page with contributor reputation scores

### Hard Difficulty

- **FE-03**: Create multi-step GrantForm with Zod validation
- **FE-06**: Add contract event streaming via SSE API route
- **FE-15**: Add Stellar Passkey (WebAuthn) sign-in flow

**Browse all issues**: [GitHub Issues](https://github.com/your-org/stellargrant-fe/issues?q=is%3Aopen+label%3Adrips-wave)

## 🎯 Wave Program Tips

1. **Claim issues early** - Comment on the issue to claim it
2. **Ask questions** - Use issue comments for clarification
3. **Show progress** - Open a draft PR early for feedback
4. **Include screenshots** - UI changes need visual proof
5. **Follow the checklist** - Complete all PR checklist items
6. **Be patient** - Reviews may take time, especially for complex changes

## 📞 Getting Help

- **GitHub Discussions** - For questions and general discussion
- **Issue Comments** - For issue-specific questions
- **Discord** - [StellarGrants Community](https://discord.gg/stellargrants) (if available)
- **Documentation** - Check the [docs](./docs/) folder first

## 🙏 Recognition

Contributors will be:

- **Listed in CONTRIBUTORS.md** (if applicable)
- **Mentioned in release notes** for significant contributions
- **Eligible for Wave Points** on completed `drips-wave` issues
- **Invited to maintainer team** for consistent high-quality contributions

## 📄 License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT License).

---

**Thank you for contributing to StellarGrants! 🌊**

Every contribution, no matter how small, helps build a better protocol for the Stellar ecosystem.
