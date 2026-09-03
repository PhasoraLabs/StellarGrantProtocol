"use client";

import React from "react";

interface PaginationProps {
  page: number;
  totalPages: number;
  onPageChange: (page: number) => void;
}

function buildPageWindow(page: number, totalPages: number): (number | "ellipsis-start" | "ellipsis-end")[] {
  if (totalPages <= 7) {
    return Array.from({ length: totalPages }, (_, i) => i + 1);
  }

  const delta = 2;
  const range: number[] = [];
  for (let i = Math.max(2, page - delta); i <= Math.min(totalPages - 1, page + delta); i++) {
    range.push(i);
  }

  const result: (number | "ellipsis-start" | "ellipsis-end")[] = [1];

  if (range[0] > 2) {
    result.push("ellipsis-start");
  }

  result.push(...range);

  if (range[range.length - 1] < totalPages - 1) {
    result.push("ellipsis-end");
  }

  result.push(totalPages);
  return result;
}

export function Pagination({ page, totalPages, onPageChange }: PaginationProps) {
  if (totalPages <= 1) return null;

  const pages = buildPageWindow(page, totalPages);

  return (
    <nav
      role="navigation"
      aria-label="Pagination"
      className="flex items-center justify-center gap-1 mt-6 flex-wrap"
    >
      {/* Previous */}
      <button
        onClick={() => onPageChange(page - 1)}
        disabled={page === 1}
        aria-label="Previous page"
        className={[
          "inline-flex items-center justify-center px-3 py-2 text-sm font-bold uppercase tracking-wider transition-all duration-300 rounded-none border",
          page === 1
            ? "border-neutral-700 text-neutral-600 cursor-not-allowed"
            : "border-accent-primary text-accent-primary hover:bg-accent-primary hover:text-bg-primary",
        ].join(" ")}
      >
        ‹ Prev
      </button>

      {/* Page buttons */}
      {pages.map((item, idx) => {
        if (item === "ellipsis-start" || item === "ellipsis-end") {
          return (
            <span
              key={`${item}-${idx}`}
              className="px-2 py-2 text-sm text-neutral-500 select-none"
              aria-hidden
            >
              …
            </span>
          );
        }

        const isActive = item === page;
        return (
          <button
            key={item}
            onClick={() => onPageChange(item)}
            aria-current={isActive ? "page" : undefined}
            aria-label={`Page ${item}`}
            className={[
              "inline-flex items-center justify-center min-w-[2.25rem] px-3 py-2 text-sm font-bold uppercase tracking-wider transition-all duration-300 rounded-none border-0",
              isActive
                ? "bg-accent-primary text-bg-primary"
                : "bg-transparent border border-accent-primary text-accent-primary hover:bg-accent-primary hover:text-bg-primary",
            ].join(" ")}
          >
            {item}
          </button>
        );
      })}

      {/* Next */}
      <button
        onClick={() => onPageChange(page + 1)}
        disabled={page === totalPages}
        aria-label="Next page"
        className={[
          "inline-flex items-center justify-center px-3 py-2 text-sm font-bold uppercase tracking-wider transition-all duration-300 rounded-none border",
          page === totalPages
            ? "border-neutral-700 text-neutral-600 cursor-not-allowed"
            : "border-accent-primary text-accent-primary hover:bg-accent-primary hover:text-bg-primary",
        ].join(" ")}
      >
        Next ›
      </button>
    </nav>
  );
}
