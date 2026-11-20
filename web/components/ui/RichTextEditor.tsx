"use client";

/**
 * RichTextEditor
 *
 * A textarea-based Markdown editor with a live sanitized preview.
 * Supports character count validation and toggle between Edit/Preview modes.
 */

import { useState } from "react";
import RichTextRenderer from "./RichTextRenderer";
import { validateDescriptionLength } from "@/lib/utils/sanitize";

interface RichTextEditorProps {
  /** Current markdown value */
  value: string;
  /** Change handler — receives raw markdown */
  onChange: (value: string) => void;
  /** Placeholder text in the editor */
  placeholder?: string;
  /** Maximum allowed characters */
  maxLength?: number;
  /** Label shown above the editor */
  label?: string;
  /** Whether the field is required */
  required?: boolean;
  /** Optional className for wrapper */
  className?: string;
}

export default function RichTextEditor({
  value,
  onChange,
  placeholder = "Describe your grant in detail. **Markdown** is supported.",
  maxLength = 5000,
  label = "Description",
  required = false,
  className = "",
}: RichTextEditorProps) {
  const [mode, setMode] = useState<"edit" | "preview">("edit");

  const validation = validateDescriptionLength(value, maxLength);
  const charCount = value.length;
  const isOverLimit = charCount > maxLength;

  return (
    <div className={`flex flex-col gap-2 ${className}`}>
      {/* Label */}
      <div className="flex items-center justify-between">
        <label className="text-sm font-medium text-white/80">
          {label}
          {required && <span className="text-red-400 ml-1">*</span>}
        </label>

        {/* Edit / Preview toggle */}
        <div className="flex rounded-lg overflow-hidden border border-white/10 text-xs">
          <button
            type="button"
            onClick={() => setMode("edit")}
            className={`px-3 py-1 transition-colors ${
              mode === "edit"
                ? "bg-indigo-600 text-white"
                : "bg-white/5 text-white/50 hover:text-white/80"
            }`}
          >
            Edit
          </button>
          <button
            type="button"
            onClick={() => setMode("preview")}
            className={`px-3 py-1 transition-colors ${
              mode === "preview"
                ? "bg-indigo-600 text-white"
                : "bg-white/5 text-white/50 hover:text-white/80"
            }`}
          >
            Preview
          </button>
        </div>
      </div>

      {/* Editor / Preview area */}
      <div className="rounded-xl border border-white/10 bg-white/5 min-h-[180px] overflow-hidden">
        {mode === "edit" ? (
          <textarea
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder={placeholder}
            rows={8}
            className={`w-full bg-transparent text-white/90 placeholder:text-white/30 
              text-sm leading-relaxed resize-y p-4 outline-none font-mono
              ${isOverLimit ? "border-red-500/60" : ""}`}
          />
        ) : (
          <div className="p-4 min-h-[180px]">
            {value.trim() === "" ? (
              <p className="text-white/30 text-sm italic">Nothing to preview yet.</p>
            ) : (
              <RichTextRenderer content={value} />
            )}
          </div>
        )}
      </div>

      {/* Footer: char count + markdown hint */}
      <div className="flex items-center justify-between text-xs text-white/40">
        <span>
          Supports{" "}
          <a
            href="https://www.markdownguide.org/cheat-sheet/"
            target="_blank"
            rel="noopener noreferrer"
            className="text-indigo-400 hover:underline"
          >
            Markdown
          </a>{" "}
          · Links, images, bold, lists, tables
        </span>
        <span className={isOverLimit ? "text-red-400 font-medium" : ""}>
          {charCount.toLocaleString()} / {maxLength.toLocaleString()}
        </span>
      </div>

      {/* Validation error */}
      {!validation.valid && value.length > 0 && (
        <p className="text-xs text-red-400">{validation.message}</p>
      )}
    </div>
  );
}
