/**
 * WalletGuard Component
 *
 * Wraps wallet-gated sections. Renders children only when a wallet is
 * connected and (optionally) the connected address satisfies a required role.
 * While the wallet session is being restored, a shimmer placeholder is shown
 * so the "connect wallet" prompt never flashes before the session resolves.
 * Wrapper component that renders children only when a wallet is connected.
 * Shows a connect prompt otherwise.
 */

"use client";

import React from "react";
import { Lock, Ban } from "lucide-react";
import type { Grant } from "@/types";
import { useWallet } from "@/hooks/useWallet";
import { useGrant } from "@/hooks/useGrant";
import { WalletConnect } from "@/components/wallet/WalletConnect";

export type WalletGuardRole = "any" | "reviewer" | "contributor" | "owner";

interface WalletGuardProps {
  children: React.ReactNode;
  requiredRole?: WalletGuardRole;
  /** Required when requiredRole is 'owner', 'reviewer', or 'contributor'. */
  grantId?: string;
  /** Custom fallback rendered instead of the default connect card when disconnected. */
  fallback?: React.ReactNode;
}

/** Roles that need grant data fetched to verify the connected address. */
function roleNeedsGrant(role: WalletGuardRole): boolean {
  return role === "owner" || role === "reviewer" || role === "contributor";
}

/**
 * Pure role check against a grant. Exported for unit testing.
 *  - owner:       address === grant.owner
 *  - reviewer:    address ∈ grant.reviewers
 *  - contributor: address === grant.recipient
 *  - any:         always authorized (handled before this is called)
 */
export function isAuthorizedForRole(
  role: WalletGuardRole,
  grant: Grant,
  address: string | null,
): boolean {
  if (!address) return false;
  switch (role) {
    case "owner":
      return grant.owner === address;
    case "reviewer":
      return grant.reviewers.includes(address);
    case "contributor":
      return grant.recipient === address;
    case "any":
    default:
      return true;
  }
}

function Shimmer() {
  return <div className="shimmer h-48 rounded-none w-full" />;
}

function ConnectCard() {
  return (
    <div className="bg-surface border border-border-color rounded-none p-8 text-center">
      <Lock className="mx-auto h-8 w-8 text-accent-primary" aria-hidden="true" />
      <h2 className="mt-4 font-orbitron text-lg font-medium text-text-primary">Wallet Required</h2>
      <p className="mt-2 font-mono text-sm text-text-muted">
        Connect your wallet to access this section.
      </p>
      <div className="mt-6 flex justify-center">
        <WalletConnect />
      </div>
    </div>
  );
}

function UnauthorizedCard() {
  return (
    <div className="bg-surface border border-border-color rounded-none p-8 text-center">
      <Ban className="mx-auto h-8 w-8 text-danger" aria-hidden="true" />
      <h2 className="mt-4 font-orbitron text-lg font-medium text-text-primary">Access Restricted</h2>
      <p className="mt-2 font-mono text-sm text-text-muted">
        Your wallet does not have access to this section.
      </p>
    </div>
  );
}

export function WalletGuard({
  children,
  requiredRole = "any",
  grantId,
  fallback,
}: WalletGuardProps) {
  const { isConnected, isConnecting, address } = useWallet();

  const needsGrant = roleNeedsGrant(requiredRole);
  // Hooks must run unconditionally; gate the fetch with `enabled`.
  const grantQuery = useGrant(grantId ?? "", {
    enabled: needsGrant && !!grantId && isConnected,
  });

  // Restoring the wallet session — avoid flashing the connect prompt.
  if (isConnecting) {
    return <Shimmer />;
  }

  if (!isConnected) {
    return fallback ? <>{fallback}</> : <ConnectCard />;
  }

  if (requiredRole !== "any") {
    // A role that needs grant context but no grantId can't be verified.
    if (needsGrant && !grantId) {
      return <UnauthorizedCard />;
    }
    if (needsGrant && grantId) {
      if (grantQuery.isLoading || !grantQuery.data) {
        return <Shimmer />;
      }
      if (!isAuthorizedForRole(requiredRole, grantQuery.data.grant, address)) {
        return <UnauthorizedCard />;
      }
    }
  }

  return <>{children}</>;
}
