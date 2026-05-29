"use client";

/**
 * ProofViewer Component
 * 
 * Premium component for viewing and interacting with submitted milestone proofs.
 * Handles IPFS CIDs (fetches gateway, renders markdown/text/images/PDFs),
 * HTTPS links (renders safe domains previews like GitHub/YouTube),
 * and raw crypto hashes (SHA-256 copyable block).
 */

import { useState, useEffect } from "react";
import RichTextRenderer from "@/components/ui/RichTextRenderer";
import { Copy, Check, ExternalLink, FileText, Image as ImageIcon, File, ShieldAlert, Eye, Terminal } from "lucide-react";

interface ProofViewerProps {
  proofHash: string;
}

interface IPFSContent {
  contentType: string;
  text: string;
}

export function ProofViewer({ proofHash }: ProofViewerProps) {
  const [currentHash, setCurrentHash] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(false);
  const [ipfsContent, setIpfsContent] = useState<IPFSContent | null>(null);
  const [copied, setCopied] = useState(false);

  const cleanProofHash = proofHash.startsWith("ipfs://")
    ? proofHash.replace("ipfs://", "")
    : proofHash;

  const isCid = cleanProofHash.startsWith("Qm") || cleanProofHash.startsWith("baf");
  const isUrl = cleanProofHash.startsWith("http://") || cleanProofHash.startsWith("https://");
  const isRawHash = /^[a-fA-F0-9]{40,}$/.test(cleanProofHash);

  // Adjust state during render when cleanProofHash changes to avoid synchronous setState inside useEffect
  if (isCid && currentHash !== cleanProofHash) {
    setCurrentHash(cleanProofHash);
    setIsLoading(true);
    setError(false);
    setIpfsContent(null);
  }

  // Copy helper
  const handleCopy = () => {
    navigator.clipboard.writeText(cleanProofHash);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Fetch IPFS CID content
  useEffect(() => {
    if (!isCid) return;

    let cancelled = false;

    // Timeout-wrapped fetch to prevent infinite hanging
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 8000);

    fetch(`https://ipfs.io/ipfs/${cleanProofHash}`, {
      method: "GET",
      signal: controller.signal,
    })
      .then(async (res) => {
        clearTimeout(timeoutId);
        if (!res.ok) throw new Error("Gateway failed to load content");

        const contentType = res.headers.get("content-type") || "text/plain";
        
        // If text, markdown, or json, we fetch the text body
        if (
          contentType.includes("text") ||
          contentType.includes("markdown") ||
          contentType.includes("json") ||
          cleanProofHash.endsWith(".txt") ||
          cleanProofHash.endsWith(".md")
        ) {
          const text = await res.text();
          return { contentType, text };
        }

        return { contentType, text: "" };
      })
      .then((content) => {
        if (!cancelled) {
          setIpfsContent(content);
          setIsLoading(false);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setError(true);
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
      controller.abort();
      clearTimeout(timeoutId);
    };
  }, [cleanProofHash, isCid]);

  // Case 1: IPFS CID Rendering
  if (isCid) {
    if (isLoading) {
      return (
        <div className="space-y-4 border border-border-color/30 p-6 bg-white/[0.01]">
          <div className="flex items-center gap-2 mb-2">
            <div className="shimmer h-4 w-32 rounded-none" />
            <div className="shimmer h-3 w-16 rounded-none" />
          </div>
          <div className="space-y-3">
            <div className="shimmer h-5 w-full rounded-none" />
            <div className="shimmer h-5 w-5/6 rounded-none" />
            <div className="shimmer h-5 w-4/5 rounded-none" />
          </div>
        </div>
      );
    }

    if (error) {
      return (
        <div className="border border-warning/40 bg-warning/5 p-6 space-y-4">
          <div className="flex items-start gap-3">
            <ShieldAlert className="h-5 w-5 text-warning flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-mono text-xs font-semibold text-warning uppercase tracking-wider">
                Content unavailable
              </p>
              <p className="text-sm text-text-muted mt-1 leading-6">
                The gateway is taking too long to fetch this proof or returned a connection error.
              </p>
            </div>
          </div>
          <a
            href={`https://ipfs.io/ipfs/${cleanProofHash}`}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5 font-mono text-xs text-accent-primary hover:text-accent-primary-hover underline transition-colors"
          >
            View on public IPFS gateway <ExternalLink className="h-3 w-3" />
          </a>
        </div>
      );
    }

    if (ipfsContent) {
      const { contentType, text } = ipfsContent;
      
      const isMarkdown =
        contentType.includes("markdown") ||
        cleanProofHash.endsWith(".md") ||
        text.trim().startsWith("#") ||
        text.includes("\n# ") ||
        text.includes("**");
        
      const isImage = contentType.startsWith("image/");
      const isPdf = contentType.includes("pdf") || cleanProofHash.endsWith(".pdf");
      const isPlainText = contentType.includes("text/plain") && !isMarkdown;

      return (
        <div className="space-y-4">
          <div className="border border-border-color/40 p-6 bg-white/[0.01]">
            <div className="flex items-center justify-between border-b border-border-color/30 pb-3 mb-4">
              <span className="font-mono text-xs text-text-muted uppercase tracking-wider flex items-center gap-1.5">
                {isMarkdown ? (
                  <>
                    <FileText className="h-4 w-4 text-accent-primary" /> Markdown Document
                  </>
                ) : isImage ? (
                  <>
                    <ImageIcon className="h-4 w-4 text-accent-primary" /> Image Evidence
                  </>
                ) : isPdf ? (
                  <>
                    <File className="h-4 w-4 text-accent-primary" /> PDF Document
                  </>
                ) : (
                  <>
                    <Terminal className="h-4 w-4 text-accent-primary" /> Plain Text Proof
                  </>
                )}
              </span>
              <a
                href={`https://ipfs.io/ipfs/${cleanProofHash}`}
                target="_blank"
                rel="noopener noreferrer"
                className="font-mono text-[10px] text-text-muted hover:text-text-primary transition-colors flex items-center gap-1"
              >
                IPFS Source <ExternalLink className="h-2.5 w-2.5" />
              </a>
            </div>

            {/* Rendering specific types */}
            {isMarkdown && (
              <div className="rich-text-container font-mono text-sm leading-relaxed">
                <RichTextRenderer content={text} />
              </div>
            )}

            {isPlainText && (
              <pre className="font-mono text-xs p-4 bg-surface border border-border-color overflow-x-auto text-text-primary leading-relaxed whitespace-pre-wrap">
                {text}
              </pre>
            )}

            {isImage && (
              <div className="flex justify-center bg-surface border border-border-color p-2 max-h-[500px] overflow-hidden">
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img
                  src={`https://ipfs.io/ipfs/${cleanProofHash}`}
                  alt="Milestone Proof"
                  className="max-w-full max-h-[480px] object-contain transition-transform hover:scale-[1.01]"
                />
              </div>
            )}

            {isPdf && (
              <div className="border border-border-color bg-surface overflow-hidden">
                <iframe
                  src={`https://ipfs.io/ipfs/${cleanProofHash}`}
                  className="w-full h-[600px] border-none"
                  title="PDF Proof Viewer"
                />
              </div>
            )}

            {!isMarkdown && !isPlainText && !isImage && !isPdf && (
              <div className="flex flex-col items-center justify-center p-8 bg-surface border border-border-color/40 text-center">
                <File className="h-12 w-12 text-text-muted/60 mb-2" />
                <p className="font-mono text-xs text-text-primary font-semibold">
                  Binary or Unknown Proof Format
                </p>
                <a
                  href={`https://ipfs.io/ipfs/${cleanProofHash}`}
                  download
                  target="_blank"
                  rel="noopener noreferrer"
                  className="mt-4 inline-flex items-center gap-2 h-9 px-4 bg-accent-primary hover:bg-accent-primary-hover text-black font-mono text-xs uppercase tracking-widest transition-colors rounded-none"
                >
                  Download Proof File <ExternalLink className="h-3.5 w-3.5 text-black" />
                </a>
              </div>
            )}
          </div>

          {/* Technical Details Disclosure */}
          <TechnicalDisclosure hash={cleanProofHash} type="IPFS CID" gatewayUrl={`https://ipfs.io/ipfs/${cleanProofHash}`} />
        </div>
      );
    }
  }

  // Case 2: HTTPS Link Rendering with known domains Safe Preview
  if (isUrl) {
    let truncatedUrl = cleanProofHash;
    if (cleanProofHash.length > 60) {
      truncatedUrl = cleanProofHash.slice(0, 57) + "...";
    }

    let isSafeDomain = false;
    let isYouTube = false;
    let youtubeVideoId = "";
    let isGitHub = false;

    try {
      const parsed = new URL(cleanProofHash);
      const hostname = parsed.hostname.toLowerCase();

      if (hostname.includes("youtube.com") || hostname.includes("youtu.be")) {
        isSafeDomain = true;
        isYouTube = true;
        if (hostname.includes("youtu.be")) {
          youtubeVideoId = parsed.pathname.slice(1);
        } else {
          youtubeVideoId = parsed.searchParams.get("v") || "";
        }
      } else if (hostname.includes("github.com")) {
        isSafeDomain = true;
        isGitHub = true;
      }
    } catch {
      // url parse error
    }

    const embedIframe = isSafeDomain ? (
      isYouTube && youtubeVideoId ? (
        <div className="aspect-video w-full bg-black border border-border-color overflow-hidden">
          <iframe
            src={`https://www.youtube.com/embed/${youtubeVideoId}`}
            className="w-full h-full border-none"
            allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
            allowFullScreen
            title="YouTube Proof Video"
          />
        </div>
      ) : isGitHub ? (
        <div className="border border-border-color bg-surface p-4 flex flex-col justify-between gap-3 bg-white/[0.01]">
          <div className="flex items-start gap-3">
            <Terminal className="h-5 w-5 text-accent-primary flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-mono text-xs font-semibold text-text-primary">
                GitHub Repository / Issue / PR Evidence
              </p>
              <p className="text-xs text-text-muted mt-1 leading-relaxed">
                External work recorded on GitHub. Click link below to view pull requests, issues, or codebase changes.
              </p>
            </div>
          </div>
        </div>
      ) : null
    ) : null;

    return (
      <div className="space-y-4">
        <div className="border border-border-color/40 p-6 bg-white/[0.01] space-y-4">
          <div className="flex items-center justify-between border-b border-border-color/30 pb-3">
            <span className="font-mono text-xs text-text-muted uppercase tracking-wider flex items-center gap-1.5">
              <Globe className="h-4 w-4 text-accent-primary" /> External Proof Link
            </span>
          </div>

          <div className="p-4 bg-surface border border-border-color flex items-center justify-between gap-4">
            <div className="overflow-hidden">
              <a
                href={cleanProofHash}
                target="_blank"
                rel="noopener noreferrer"
                className="font-mono text-sm text-accent-primary hover:text-accent-primary-hover underline break-all inline-flex items-center gap-1"
              >
                {truncatedUrl}
                <ExternalLink className="h-3 w-3 flex-shrink-0" />
              </a>
            </div>
          </div>

          {/* Render embed preview if safe domain */}
          {isSafeDomain && embedIframe}
        </div>

        <TechnicalDisclosure hash={cleanProofHash} type="HTTPS URL" />
      </div>
    );
  }

  // Case 3: Raw Cryptographic Hash (SHA-256)
  if (isRawHash || cleanProofHash.length >= 40) {
    return (
      <div className="space-y-4">
        <div className="border border-border-color/40 p-6 bg-white/[0.01] space-y-4">
          <div className="flex items-center justify-between border-b border-border-color/30 pb-3">
            <span className="font-mono text-xs text-text-muted uppercase tracking-wider flex items-center gap-1.5">
              <Terminal className="h-4 w-4 text-accent-primary" /> Proof Hash (SHA-256)
            </span>
          </div>

          <div className="flex flex-col gap-2">
            <div className="relative flex items-center bg-surface border border-border-color">
              <div className="flex-1 font-mono text-xs p-4 overflow-x-auto text-text-primary whitespace-nowrap select-all scrollbar-thin">
                {cleanProofHash}
              </div>
              <button
                type="button"
                onClick={handleCopy}
                className="p-4 hover:bg-white/5 border-l border-border-color text-text-muted hover:text-text-primary transition-all flex items-center justify-center flex-shrink-0"
                title="Copy Hash"
              >
                {copied ? <Check className="h-4 w-4 text-success" /> : <Copy className="h-4 w-4" />}
              </button>
            </div>
            
            {copied && (
              <span className="font-mono text-[10px] text-success self-end mr-2 animate-pulse">
                Copied to clipboard!
              </span>
            )}
          </div>
        </div>

        <TechnicalDisclosure hash={cleanProofHash} type="Cryptographic Hash (SHA-256)" />
      </div>
    );
  }

  // Fallback default
  return (
    <div className="space-y-4">
      <div className="border border-border-color/40 p-6 bg-white/[0.01]">
        <p className="font-mono text-xs text-text-primary mb-2">Unrecognized Proof Format</p>
        <p className="font-mono text-sm text-text-muted break-all">{cleanProofHash}</p>
      </div>
      <TechnicalDisclosure hash={cleanProofHash} type="Unknown Format" />
    </div>
  );
}

// ── Technical Disclosure Helper ─────────────────────────────────────────────
interface TechnicalDisclosureProps {
  hash: string;
  type: string;
  gatewayUrl?: string;
}

function TechnicalDisclosure({ hash, type, gatewayUrl }: TechnicalDisclosureProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(hash);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <details className="group border border-border-color/20 bg-white/[0.005] hover:bg-white/[0.01] transition-all rounded-none">
      <summary className="flex items-center justify-between font-mono text-[11px] text-text-muted cursor-pointer p-3 select-none uppercase tracking-wider font-semibold">
        <span className="flex items-center gap-1.5">
          <Eye className="h-3.5 w-3.5 text-text-muted/80" /> Technical Details
        </span>
        <span className="text-[10px] opacity-60 transition-transform group-open:rotate-180">▼</span>
      </summary>
      
      <div className="px-3 pb-3 pt-1 border-t border-border-color/10 font-mono text-[11px] text-text-muted space-y-2 leading-relaxed">
        <div className="flex justify-between gap-4 py-1 border-b border-border-color/5">
          <span>Format Type:</span>
          <span className="text-text-primary font-semibold">{type}</span>
        </div>
        <div className="flex justify-between gap-4 py-1 border-b border-border-color/5">
          <span>Raw Value:</span>
          <div className="flex items-center gap-1 max-w-[70%]">
            <span className="text-text-primary truncate select-all">{hash}</span>
            <button
              onClick={handleCopy}
              className="p-1 hover:bg-white/10 rounded transition-all text-text-muted hover:text-text-primary"
              title="Copy details"
            >
              {copied ? <Check className="h-3 w-3 text-success" /> : <Copy className="h-3 w-3" />}
            </button>
          </div>
        </div>
        {gatewayUrl && (
          <div className="flex justify-between gap-4 py-1 border-b border-border-color/5">
            <span>Direct Gateway URL:</span>
            <a
              href={gatewayUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-accent-primary hover:underline flex items-center gap-1 truncate max-w-[70%]"
            >
              {gatewayUrl} <ExternalLink className="h-2.5 w-2.5" />
            </a>
          </div>
        )}
      </div>
    </details>
  );
}

// ── Small Inline Globe Icon fallback ──────────────────────────────────────────
function Globe({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
    >
      <circle cx="12" cy="12" r="10" />
      <path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" />
      <path d="M2 12h20" />
    </svg>
  );
}
