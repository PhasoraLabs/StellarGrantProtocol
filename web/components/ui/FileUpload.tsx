"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, CheckCircle, Copy, Upload, X } from "lucide-react";
import { useIPFS } from "@/hooks/useIPFS";

export interface FileUploadProps {
  onFileSelected: (file: File) => void;
  onCidReady?: (cid: string) => void;
  autoUpload?: boolean;
  accept?: string;
  maxSizeMB?: number;
  label?: string;
  disabled?: boolean;
  existingCid?: string;
}

type UploadState = "idle" | "dragover" | "selected" | "uploading" | "done" | "error";

export function FileUpload({
  onFileSelected,
  onCidReady,
  autoUpload = false,
  accept,
  maxSizeMB = 10,
  label,
  disabled = false,
  existingCid,
}: FileUploadProps) {
  const [uploadState, setUploadState] = useState<UploadState>(existingCid ? "done" : "idle");
  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [cid, setCid] = useState<string | null>(existingCid ?? null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const previewUrlRef = useRef<string | null>(null);
  const cancelledRef = useRef(false);
  const { upload } = useIPFS();

  useEffect(() => {
    return () => {
      if (previewUrlRef.current) URL.revokeObjectURL(previewUrlRef.current);
    };
  }, []);

  const validate = useCallback(
    (f: File): string | null => {
      if (f.size > maxSizeMB * 1024 * 1024) return `File exceeds ${maxSizeMB} MB limit`;
      if (accept) {
        const accepted = accept.split(",").map((s) => s.trim());
        const matches = accepted.some((a) => {
          if (a.startsWith(".")) return f.name.toLowerCase().endsWith(a.toLowerCase());
          if (a.endsWith("/*")) return f.type.startsWith(a.replace("/*", "/"));
          return f.type === a;
        });
        if (!matches) return "File type not accepted";
      }
      return null;
    },
    [accept, maxSizeMB],
  );

  const doUpload = useCallback(
    async (f: File) => {
      cancelledRef.current = false;
      setUploadState("uploading");
      try {
        const result = await upload(f);
        if (cancelledRef.current) return;
        if (result) {
          setCid(result);
          setUploadState("done");
          onCidReady?.(result);
        } else {
          throw new Error("Upload returned an empty CID");
        }
      } catch (e) {
        if (cancelledRef.current) return;
        setError(e instanceof Error ? e.message : "Upload failed");
        setUploadState("error");
      }
    },
    [upload, onCidReady],
  );

  const handleFile = useCallback(
    (f: File) => {
      const validationError = validate(f);
      if (validationError) {
        setError(validationError);
        setUploadState("error");
        return;
      }

      setFile(f);
      setError(null);

      if (previewUrlRef.current) URL.revokeObjectURL(previewUrlRef.current);
      if (f.type.startsWith("image/")) {
        const url = URL.createObjectURL(f);
        previewUrlRef.current = url;
        setPreview(url);
      } else {
        setPreview(null);
      }

      onFileSelected(f);

      if (autoUpload) {
        void doUpload(f);
      } else {
        setUploadState("selected");
      }
    },
    [validate, onFileSelected, autoUpload, doUpload],
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      if (disabled) return;
      const f = e.dataTransfer.files[0];
      if (f) handleFile(f);
    },
    [disabled, handleFile],
  );

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    if (!disabled && uploadState !== "uploading" && uploadState !== "done") {
      setUploadState("dragover");
    }
  };

  const handleDragLeave = () => {
    if (uploadState === "dragover") setUploadState("idle");
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (f) handleFile(f);
    e.target.value = "";
  };

  const handleCopy = async () => {
    if (!cid) return;
    await navigator.clipboard.writeText(cid);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleReset = () => {
    cancelledRef.current = true;
    if (previewUrlRef.current) {
      URL.revokeObjectURL(previewUrlRef.current);
      previewUrlRef.current = null;
    }
    setUploadState("idle");
    setFile(null);
    setPreview(null);
    setCid(null);
    setError(null);
  };

  const zoneClass = [
    "relative flex min-h-[140px] flex-col items-center justify-center gap-3 border-2 border-dashed p-8 text-center transition-all duration-200",
    disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer",
    uploadState === "dragover"
      ? "border-accent-secondary bg-accent-secondary/5"
      : uploadState === "done"
        ? "border-success/40 bg-success/5"
        : uploadState === "error"
          ? "border-danger/40 bg-danger/5"
          : "border-border-color bg-bg-secondary hover:border-accent-secondary/60",
  ].join(" ");

  const openPicker = () => {
    if (!disabled && uploadState !== "uploading" && uploadState !== "done") {
      inputRef.current?.click();
    }
  };

  return (
    <div className="w-full">
      {label && (
        <label className="mb-2 block font-mono text-xs uppercase tracking-[0.2em] text-text-muted">
          {label}
        </label>
      )}
      <div
        className={zoneClass}
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onClick={openPicker}
        role="button"
        tabIndex={disabled ? -1 : 0}
        onKeyDown={(e) => {
          if (e.key === "Enter") openPicker();
        }}
        aria-label="File upload zone"
      >
        <input
          ref={inputRef}
          type="file"
          accept={accept}
          className="sr-only"
          onChange={handleInputChange}
          disabled={disabled}
        />

        {(uploadState === "idle" || uploadState === "dragover") && (
          <>
            <Upload className="h-8 w-8 text-text-muted" />
            <div>
              <p className="font-mono text-sm text-text-muted">
                Drag &amp; drop your proof here
              </p>
              <p className="mt-1 font-mono text-xs text-text-muted/60">
                or <span className="text-accent-secondary underline">Browse files</span>
              </p>
            </div>
            <p className="font-mono text-xs text-text-muted/50">
              {accept ? accept.replace(/,\s*/g, " · ") : ".md · .txt · .pdf · images"}&nbsp; Max:{" "}
              {maxSizeMB} MB
            </p>
          </>
        )}

        {uploadState === "selected" && file && (
          <div
            className="flex w-full items-start gap-4"
            onClick={(e) => e.stopPropagation()}
          >
            {preview && (
              // eslint-disable-next-line @next/next/no-img-element
              <img src={preview} alt="preview" className="h-24 w-24 shrink-0 object-cover" />
            )}
            <div className="flex-1 text-left">
              <p className="truncate font-mono text-sm text-text-primary">{file.name}</p>
              <p className="font-mono text-xs text-text-muted">
                {(file.size / 1024 / 1024).toFixed(2)} MB · {file.type || "unknown type"}
              </p>
              <button
                className="mt-3 inline-flex items-center gap-2 bg-accent-primary px-4 py-1.5 font-orbitron text-xs font-bold uppercase tracking-wider text-bg-primary transition-all hover:opacity-90"
                onClick={() => void doUpload(file)}
              >
                <Upload className="h-3 w-3" />
                Upload to IPFS
              </button>
            </div>
            <button
              className="shrink-0 text-text-muted hover:text-danger"
              onClick={handleReset}
              aria-label="Remove file"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        )}

        {uploadState === "uploading" && (
          <div className="w-full" onClick={(e) => e.stopPropagation()}>
            <p className="mb-3 text-center font-mono text-sm text-text-muted">
              Uploading to IPFS…
            </p>
            <div className="h-1.5 w-full overflow-hidden bg-border-color">
              <div className="shimmer h-full w-full" />
            </div>
            <div className="mt-3 flex justify-center">
              <button
                className="font-mono text-xs text-text-muted underline hover:text-danger"
                onClick={handleReset}
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {uploadState === "done" && cid && (
          <div
            className="flex w-full items-start gap-4"
            onClick={(e) => e.stopPropagation()}
          >
            {preview && (
              // eslint-disable-next-line @next/next/no-img-element
              <img src={preview} alt="preview" className="h-24 w-24 shrink-0 object-cover" />
            )}
            <div className="min-w-0 flex-1 text-left">
              <div className="mb-1 flex items-center gap-2">
                <CheckCircle className="h-4 w-4 shrink-0 text-success" />
                <p className="font-mono text-xs text-success">Uploaded to IPFS</p>
              </div>
              <div className="flex items-center gap-2">
                <p className="truncate font-mono text-xs text-text-muted">
                  {cid.length > 20 ? `${cid.slice(0, 10)}…${cid.slice(-8)}` : cid}
                </p>
                <button
                  onClick={() => void handleCopy()}
                  className="shrink-0 text-text-muted hover:text-accent-secondary"
                  aria-label="Copy CID"
                >
                  <Copy className="h-3 w-3" />
                </button>
                {copied && <span className="font-mono text-xs text-success">Copied!</span>}
              </div>
            </div>
            <button
              className="shrink-0 text-text-muted hover:text-text-primary"
              onClick={handleReset}
              aria-label="Replace file"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        )}

        {uploadState === "error" && (
          <div className="w-full text-center" onClick={(e) => e.stopPropagation()}>
            <div className="mb-1 flex items-center justify-center gap-2">
              <AlertTriangle className="h-4 w-4 text-danger" />
              <p className="font-mono text-xs text-danger">{error}</p>
            </div>
            <button
              className="mt-2 font-mono text-xs text-text-muted underline hover:text-danger"
              onClick={(e) => {
                e.stopPropagation();
                setUploadState("idle");
                setError(null);
                setFile(null);
                setPreview(null);
              }}
            >
              Try again
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
