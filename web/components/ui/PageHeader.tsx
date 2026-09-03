import React from "react";

export interface PageHeaderProps {
  eyebrow?: string;
  title: string;
  description?: string;
  action?: React.ReactNode;
}

export function PageHeader({ eyebrow, title, description, action }: PageHeaderProps) {
  return (
    <div className="mb-8 flex flex-wrap items-end justify-between gap-4">
      <div>
        {eyebrow && (
          <p className="font-mono text-xs uppercase tracking-[0.32em] text-accent-secondary">
            {eyebrow}
          </p>
        )}
        <h1 className="mt-3 text-3xl font-bold">{title}</h1>
        {description && (
          <p className="mt-2 max-w-2xl text-sm leading-6 text-text-muted">{description}</p>
        )}
      </div>
      {action && <div>{action}</div>}
    </div>
  );
}
