"use client";

/**
 * MilestoneSubmitForm Component
 * 
 * Form for contributors to submit milestone proof.
 * Accepts text description and IPFS file upload.
 * Builds and submits the milestone_submit() contract transaction.
 */

import { useState, useRef } from "react";
import { z } from "zod";
import { xdr, Address, nativeToScVal } from "@stellar/stellar-sdk";
import { useWallet } from "@/hooks/useWallet";
import { useIPFS } from "@/hooks/useIPFS";
import { useContractTransaction } from "@/hooks/useContractTransaction";
import { useGrant } from "@/hooks/useGrant";
import { useMilestone } from "@/hooks/useMilestone";
import { Upload, X, Check, Loader2 } from "lucide-react";

interface MilestoneSubmitFormProps {
  grantId: string;
  milestoneIdx: number;
  onSuccess?: () => void;
}

const formSchema = z.object({
  description: z.string().min(50, "Describe your work in at least 50 characters"),
  proofUrl: z.string().url("Must be a valid URL").or(z.literal("")).optional(),
  file: z.any().optional(),
}).refine((data) => {
  const hasUrl = !!data.proofUrl && data.proofUrl.trim() !== "";
  const hasFile = !!data.file;
  return hasUrl || hasFile;
}, {
  message: "Provide a proof URL or upload a file",
  path: ["proofUrl"],
});

export function MilestoneSubmitForm({
  grantId,
  milestoneIdx,
  onSuccess,
}: MilestoneSubmitFormProps) {
  const { address } = useWallet();
  const { data: grant, isLoading: grantLoading } = useGrant(grantId);
  const { milestone, isLoading: milestoneLoading, refetch: refetchMilestone } = useMilestone(grantId, milestoneIdx);

  const [description, setDescription] = useState("");
  const [proofUrl, setProofUrl] = useState("");
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  
  const [formErrors, setFormErrors] = useState<Record<string, string>>({});
  const [isDragActive, setIsDragActive] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const ipfs = useIPFS();
  const tx = useContractTransaction();

  // Guard: Check if wallet is recipient and milestone is pending (not submitted/approved/paid)
  const isRecipient = !!address && !!grant && address === grant.grant.owner;
  const isPending = !!milestone && !milestone.submitted && !milestone.approved && !milestone.paid;

  if (grantLoading || milestoneLoading) {
    return (
      <div className="space-y-4 animate-pulse p-4">
        <div className="h-4 bg-white/10 w-1/4 rounded-none" />
        <div className="h-32 bg-white/10 w-full rounded-none" />
        <div className="h-10 bg-white/10 w-1/3 rounded-none" />
      </div>
    );
  }

  // Guard rendering
  if (!isRecipient || !isPending) {
    return null;
  }

  // Handle Drag & Drop
  const handleDrag = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === "dragenter" || e.type === "dragover") {
      setIsDragActive(true);
    } else if (e.type === "dragleave") {
      setIsDragActive(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragActive(false);

    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      const file = e.dataTransfer.files[0];
      // Max 10MB
      if (file.size > 10 * 1024 * 1024) {
        setFormErrors((prev) => ({ ...prev, file: "File exceeds 10 MB limit" }));
        return;
      }
      setSelectedFile(file);
      setFormErrors((prev) => {
        const copy = { ...prev };
        delete copy.file;
        delete copy.proofUrl; // clear mutual exclusivity error
        return copy;
      });
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      const file = e.target.files[0];
      if (file.size > 10 * 1024 * 1024) {
        setFormErrors((prev) => ({ ...prev, file: "File exceeds 10 MB limit" }));
        return;
      }
      setSelectedFile(file);
      setFormErrors((prev) => {
        const copy = { ...prev };
        delete copy.file;
        delete copy.proofUrl;
        return copy;
      });
    }
  };

  const removeFile = () => {
    setSelectedFile(null);
    ipfs.reset();
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  // Submit Flow
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormErrors({});
    tx.reset();

    // 1. Zod Validation
    const validation = formSchema.safeParse({
      description,
      proofUrl,
      file: selectedFile,
    });

    if (!validation.success) {
      const errors: Record<string, string> = {};
      validation.error.issues.forEach((issue) => {
        const field = issue.path[0] as string;
        errors[field] = issue.message;
      });
      setFormErrors(errors);
      return;
    }

    try {
      let finalProofHash = "";

      // 2. Upload to IPFS if file is selected
      if (selectedFile) {
        const ipfsCid = await ipfs.upload(selectedFile);
        if (!ipfsCid) {
          throw new Error("IPFS upload failed");
        }
        finalProofHash = ipfsCid;
      } else {
        // 3. Hash URL with SHA-256 if no file
        const msgBuffer = new TextEncoder().encode(proofUrl);
        const hashBuffer = await crypto.subtle.digest("SHA-256", msgBuffer);
        const hashArray = Array.from(new Uint8Array(hashBuffer));
        finalProofHash = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
      }

      if (!finalProofHash) {
        throw new Error("Failed to generate proof hash");
      }

      // 4. Build Soroban ScVals for milestone_submit(grant_id, milestone_idx, recipient, description, proof_url, payout_token)
      if (!address) throw new Error("Wallet not connected");

      const grantIdBig = BigInt(grantId);
      const milestoneIdxU32 = milestoneIdx;
      const recipientAddr = Address.fromString(address);

      const scArgs = [
        nativeToScVal(grantIdBig),
        xdr.ScVal.scvU32(milestoneIdxU32),
        recipientAddr.toScVal(),
        nativeToScVal(description),
        nativeToScVal(selectedFile ? `ipfs://${finalProofHash}` : proofUrl),
        nativeToScVal(null), // Option<Address> -> None (scvVoid)
      ];

      // Execute transaction via the hook
      // Try both possible contract method naming conventions
      const txHash = await tx.execute({
        method: "milestone_submit",
        args: scArgs,
        onSuccess: async () => {
          await refetchMilestone();
          onSuccess?.();
        },
      });

      if (!txHash) {
        // Fallback for cases where contract uses camelCase bindings in wrapper methods
        await tx.execute({
          method: "milestoneSubmit",
          args: scArgs,
          onSuccess: async () => {
            await refetchMilestone();
            onSuccess?.();
          },
        });
      }

    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : "Submission failed";
      setFormErrors((prev) => ({ ...prev, general: msg }));
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Description Textarea */}
      <div className="space-y-2">
        <label className="block font-mono text-xs uppercase tracking-widest text-text-muted">
          Describe your work *
        </label>
        <textarea
          value={description}
          onChange={(e) => {
            setDescription(e.target.value);
            if (e.target.value.length >= 50) {
              setFormErrors((prev) => {
                const copy = { ...prev };
                delete copy.description;
                return copy;
              });
            }
          }}
          disabled={ipfs.isUploading || tx.isPending || tx.isSimulating}
          placeholder="Provide a detailed description of what you accomplished in this milestone (minimum 50 characters)..."
          className={`w-full min-h-[140px] px-4 py-3 bg-surface border font-mono text-sm text-text-primary rounded-none focus:outline-none transition-all placeholder:text-text-muted/40 ${
            formErrors.description ? "border-danger focus:border-danger" : "border-border-color focus:border-accent-primary"
          }`}
        />
        {formErrors.description && (
          <p className="font-mono text-xs text-danger">{formErrors.description}</p>
        )}
      </div>

      {/* Proof URL Input */}
      <div className="space-y-2">
        <label className="block font-mono text-xs uppercase tracking-widest text-text-muted">
          Proof URL (e.g. GitHub Pull Request, Demo Video)
        </label>
        <input
          type="text"
          value={proofUrl}
          onChange={(e) => {
            setProofUrl(e.target.value);
            setFormErrors((prev) => {
              const copy = { ...prev };
              delete copy.proofUrl;
              return copy;
            });
          }}
          disabled={!!selectedFile || ipfs.isUploading || tx.isPending || tx.isSimulating}
          placeholder={selectedFile ? "URL disabled while file is uploaded" : "https://github.com/..."}
          className={`w-full h-11 px-4 bg-surface border font-mono text-sm text-text-primary rounded-none focus:outline-none transition-all placeholder:text-text-muted/40 ${
            formErrors.proofUrl ? "border-danger focus:border-danger" : "border-border-color focus:border-accent-primary"
          } ${selectedFile ? "opacity-50 cursor-not-allowed" : ""}`}
        />
        {formErrors.proofUrl && (
          <p className="font-mono text-xs text-danger">{formErrors.proofUrl}</p>
        )}
      </div>

      {/* Divider */}
      {!proofUrl && !selectedFile && (
        <div className="flex items-center justify-center gap-4 my-2">
          <div className="h-[1px] bg-border-color/30 flex-1" />
          <span className="font-mono text-xs text-text-muted/60 uppercase">OR</span>
          <div className="h-[1px] bg-border-color/30 flex-1" />
        </div>
      )}

      {/* File Upload Zone */}
      <div className="space-y-2">
        <label className="block font-mono text-xs uppercase tracking-widest text-text-muted">
          Upload file evidence (.md, .txt, .pdf, images · Max 10MB)
        </label>
        
        {!selectedFile ? (
          <div
            onDragEnter={handleDrag}
            onDragLeave={handleDrag}
            onDragOver={handleDrag}
            onDrop={handleDrop}
            onClick={() => fileInputRef.current?.click()}
            className={`flex flex-col items-center justify-center border-2 border-dashed rounded-none p-8 text-center cursor-pointer transition-all hover:bg-white/[0.02] ${
              isDragActive ? "border-accent-primary bg-white/[0.04]" : "border-border-color/60"
            } ${proofUrl ? "opacity-50 cursor-not-allowed pointer-events-none" : ""}`}
          >
            <input
              type="file"
              ref={fileInputRef}
              onChange={handleFileChange}
              accept=".md,.txt,.pdf,image/*"
              className="hidden"
              disabled={!!proofUrl}
            />
            <Upload className="h-8 w-8 text-text-muted/60 mb-2" />
            <p className="font-mono text-xs text-text-primary font-semibold">
              Drag & Drop file here or <span className="text-accent-primary underline">browse</span>
            </p>
            <p className="font-mono text-[10px] text-text-muted mt-1">
              Supports markdown, text, pdf, or png/jpg images up to 10 MB
            </p>
          </div>
        ) : (
          <div className="flex items-center justify-between border border-border-color p-4 bg-white/[0.02]">
            <div className="flex items-center gap-3 overflow-hidden">
              <div className="flex items-center justify-center h-10 w-10 bg-accent-primary/10 border border-accent-primary/20 text-accent-primary">
                <Check className="h-5 w-5" />
              </div>
              <div className="overflow-hidden">
                <p className="font-mono text-xs text-text-primary truncate font-semibold">
                  {selectedFile.name}
                </p>
                <p className="font-mono text-[10px] text-text-muted">
                  {(selectedFile.size / 1024 / 1024).toFixed(2)} MB
                </p>
              </div>
            </div>
            
            <button
              type="button"
              onClick={removeFile}
              disabled={ipfs.isUploading || tx.isPending || tx.isSimulating}
              className="p-1 hover:bg-white/10 text-text-muted hover:text-text-primary transition-all rounded-none"
            >
              <X className="h-5 w-5" />
            </button>
          </div>
        )}

        {/* IPFS Upload Progress Bar */}
        {ipfs.isUploading && (
          <div className="space-y-1.5 p-3 bg-white/[0.01] border border-border-color/30">
            <div className="flex items-center justify-between font-mono text-[10px] text-text-muted">
              <span>Uploading evidence to IPFS...</span>
              <span>{ipfs.uploadProgress}%</span>
            </div>
            <div className="w-full h-1 bg-border-color/30 overflow-hidden">
              <div
                className="h-full bg-accent-primary transition-all duration-150"
                style={{ width: `${ipfs.uploadProgress}%` }}
              />
            </div>
          </div>
        )}

        {ipfs.cid && (
          <div className="p-3 bg-success/10 border border-success/30">
            <p className="font-mono text-[10px] text-success">
              ✓ Successfully pinned to IPFS:{" "}
              <span className="font-semibold select-all">{ipfs.cid}</span>
            </p>
          </div>
        )}

        {formErrors.file && (
          <p className="font-mono text-xs text-danger">{formErrors.file}</p>
        )}
      </div>

      {/* General Submission State / Errors */}
      {tx.error && (
        <div className="p-4 bg-danger/10 border border-danger/30">
          <p className="font-mono text-xs text-danger">Error: {tx.error}</p>
        </div>
      )}
      {formErrors.general && (
        <div className="p-4 bg-danger/10 border border-danger/30">
          <p className="font-mono text-xs text-danger">Error: {formErrors.general}</p>
        </div>
      )}

      {/* Submit Button */}
      <button
        type="submit"
        disabled={
          ipfs.isUploading ||
          tx.isPending ||
          tx.isSimulating ||
          (!description || (!proofUrl && !selectedFile))
        }
        className="w-full h-11 bg-accent-primary hover:bg-accent-primary-hover disabled:bg-white/10 disabled:cursor-not-allowed font-mono text-xs uppercase tracking-widest text-black font-bold transition-all flex items-center justify-center gap-2 rounded-none"
      >
        {(tx.isPending || tx.isSimulating) && (
          <Loader2 className="h-4 w-4 animate-spin text-black" />
        )}
        {tx.isSimulating
          ? "Simulating Transaction..."
          : tx.isPending
          ? "Submitting Proof..."
          : "Submit Milestone"}
      </button>
    </form>
  );
}
