export type ToastVariant = "success" | "error" | "warning" | "info";

export interface ToastOptions {
  title: string;
  description?: string;
  variant?: ToastVariant;
  duration?: number;
  action?: {
    label: string;
    href?: string;
    onClick?: () => void;
  };
}

export interface InternalToast extends Required<Omit<ToastOptions, "action">> {
  id: string;
  action?: {
    label: string;
    href?: string;
    onClick?: () => void;
  };
}

const TOAST_EVENT = "stellargrant:toast";

export function toast(options: ToastOptions) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(
    new CustomEvent<ToastOptions>(TOAST_EVENT, { detail: options })
  );
}

export function toastSuccess(
  title: string,
  opts?: Omit<ToastOptions, "variant" | "title">
) {
  toast({ title, variant: "success", ...opts });
}

export function toastError(
  title: string,
  opts?: Omit<ToastOptions, "variant" | "title">
) {
  toast({ title, variant: "error", ...opts });
}

export { TOAST_EVENT };
