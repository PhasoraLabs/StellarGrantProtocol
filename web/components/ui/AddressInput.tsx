"use client";

import { useCallback, useId, useRef, useState } from "react";
import { useWalletStore } from "@/lib/store";

const STELLAR_ADDRESS_REGEX = /^G[A-Z2-7]{55}$/;
const INVALID_MESSAGE = "Invalid Stellar address";

export function addressToColor(addr: string): string {
  let hash = 0;
  for (const char of addr) hash = char.charCodeAt(0) + ((hash << 5) - hash);
  return `hsl(${Math.abs(hash) % 360}, 70%, 50%)`;
}

function isValidStellarAddress(value: string): boolean {
  return STELLAR_ADDRESS_REGEX.test(value);
}

function CheckIcon() {
  return (
    <svg
      aria-hidden="true"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="text-success"
    >
      <polyline points="20 6 9 17 4 12" />
    </svg>
  );
}

function XIcon() {
  return (
    <svg
      aria-hidden="true"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="text-danger"
    >
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </svg>
  );
}

export interface AddressInputProps {
  value: string;
  onChange: (value: string) => void;
  label?: string;
  placeholder?: string;
  error?: string;
  showUseMyAddress?: boolean;
  showAvatar?: boolean;
  disabled?: boolean;
  onValidAddress?: (address: string) => void;
}

export function AddressInput({
  value,
  onChange,
  label,
  placeholder,
  error: externalError,
  showUseMyAddress = false,
  showAvatar = false,
  disabled = false,
  onValidAddress,
}: AddressInputProps) {
  const inputId = useId();
  const walletAddress = useWalletStore((s) => s.address);
  const [validation, setValidation] = useState<"valid" | "invalid" | null>(null);
  const pastedInvalidRef = useRef(false);
  const skipValidationResetRef = useRef(false);

  const applyValid = useCallback(
    (address: string) => {
      setValidation("valid");
      onValidAddress?.(address);
    },
    [onValidAddress],
  );

  const validateValue = useCallback(
    (next: string) => {
      if (!next) {
        setValidation(null);
        pastedInvalidRef.current = false;
        return;
      }

      if (isValidStellarAddress(next)) {
        applyValid(next);
        pastedInvalidRef.current = false;
        return;
      }

      setValidation("invalid");
      pastedInvalidRef.current = false;
    },
    [applyValid],
  );

  const handleChange = (next: string) => {
    onChange(next);
    if (skipValidationResetRef.current) {
      skipValidationResetRef.current = false;
      return;
    }
    setValidation(null);
    pastedInvalidRef.current = false;
  };

  const handleBlur = () => {
    if (!value) {
      setValidation(null);
      pastedInvalidRef.current = false;
      return;
    }
    validateValue(value);
  };

  const handlePaste = (e: React.ClipboardEvent<HTMLInputElement>) => {
    const pasted = e.clipboardData.getData("text").trim();
    if (!pasted) return;

    if (isValidStellarAddress(pasted)) {
      pastedInvalidRef.current = false;
      skipValidationResetRef.current = true;
      applyValid(pasted);
      return;
    }

    pastedInvalidRef.current = true;
    setValidation(null);
  };

  const handleUseMyAddress = () => {
    if (!walletAddress) return;
    onChange(walletAddress);
    applyValid(walletAddress);
    pastedInvalidRef.current = false;
  };

  const showValid = validation === "valid";
  const showInvalid = validation === "invalid";
  const displayError =
    externalError ?? (showInvalid ? INVALID_MESSAGE : undefined);
  const showIdenticon = showAvatar && showValid && value;

  const borderClass = showValid
    ? "border-success"
    : showInvalid
      ? "border-danger"
      : "border-border-color";

  return (
    <div className="space-y-1">
      {label && (
        <label htmlFor={inputId} className="block font-mono text-xs text-text-muted">
          {label}
        </label>
      )}

      <div className="relative flex items-center">
        {showIdenticon && (
          <div
            className="absolute left-2 z-10 flex h-6 w-6 shrink-0 items-center justify-center font-mono text-[10px] font-bold text-white"
            style={{ backgroundColor: addressToColor(value) }}
            aria-hidden="true"
          >
            {value.slice(0, 2)}
          </div>
        )}

        <input
          id={inputId}
          type="text"
          value={value}
          disabled={disabled}
          placeholder={placeholder}
          onChange={(e) => handleChange(e.target.value)}
          onBlur={handleBlur}
          onPaste={handlePaste}
          className={[
            "w-full rounded-none border bg-surface py-2 pr-9 font-mono text-sm text-text-primary outline-none transition-colors",
            showIdenticon ? "pl-10" : "px-3",
            borderClass,
            disabled ? "cursor-not-allowed opacity-60" : "",
          ].join(" ")}
          aria-invalid={showInvalid || Boolean(externalError) || undefined}
          aria-describedby={displayError ? `${inputId}-error` : undefined}
        />

        {(showValid || showInvalid) && (
          <span className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2">
            {showValid ? <CheckIcon /> : <XIcon />}
          </span>
        )}
      </div>

      {displayError && (
        <p id={`${inputId}-error`} className="font-mono text-xs text-danger">
          {displayError}
        </p>
      )}

      {showUseMyAddress && walletAddress && (
        <button
          type="button"
          onClick={handleUseMyAddress}
          disabled={disabled}
          className="font-mono text-xs text-accent-primary underline-offset-2 hover:underline disabled:cursor-not-allowed disabled:opacity-60"
        >
          Use my address
        </button>
      )}
    </div>
  );
}
