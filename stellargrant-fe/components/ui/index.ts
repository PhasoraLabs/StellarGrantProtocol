/**
 * UI Component Barrel Export
 *
 * Import all shared UI primitives from this single entry-point:
 *   import { Badge, Button, StatusDot } from "@/components/ui";
 */

export { Badge } from "./Badge";
export type { BadgeProps, BadgeVariant, BadgeSize } from "./Badge";

export { Button } from "./Button";
export { Card } from "./Card";
export { NotificationToast } from "./NotificationToast";
export { Pagination } from "./Pagination";
export { default as RichTextEditor } from "./RichTextEditor";
export { default as RichTextRenderer } from "./RichTextRenderer";
export { StatBadge } from "./StatBadge";
export { StatusDot } from "./StatusDot";
export { PageTransition } from "./PageTransition";

export { AddressInput, addressToColor } from "./AddressInput";
export type { AddressInputProps } from "./AddressInput";

export { EmptyState } from "./EmptyState";
export type { EmptyStateProps } from "./EmptyState";

export { ErrorCard } from "./ErrorCard";
export type { ErrorCardProps } from "./ErrorCard";

export { PageHeader } from "./PageHeader";
export type { PageHeaderProps } from "./PageHeader";

export { FileUpload } from "./FileUpload";
export type { FileUploadProps } from "./FileUpload";

export { QRCode } from "./QRCode";
export type { QRCodeProps } from "./QRCode";
export { ConfirmationDialog } from "./ConfirmationDialog";
export type { ConfirmationDialogProps } from "./ConfirmationDialog";

export { DatePicker, computeRelativeDateLabel, todayIso } from "./DatePicker";
export type { DatePickerProps, RelativeDateLabel, RelativeDateTone } from "./DatePicker";
