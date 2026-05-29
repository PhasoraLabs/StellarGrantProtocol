"use client";

import { useState } from "react";

export function useIPFS() {
  const [isUploading, setIsUploading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [cid, setCid] = useState<string | null>(null);
  const [error, setError] = useState<Error | null>(null);

  const upload = async (file: File | object): Promise<string> => {
    setIsUploading(true);
    setProgress(0);
    setError(null);

    return new Promise((resolve) => {
      const duration = 1200; // 1.2s upload animation
      const intervalTime = 80;
      const steps = duration / intervalTime;
      let currentStep = 0;

      const timer = setInterval(() => {
        currentStep++;
        const currentProgress = Math.min(Math.round((currentStep / steps) * 100), 95);
        setProgress(currentProgress);
      }, intervalTime);

      setTimeout(() => {
        clearInterval(timer);
        setProgress(100);
        setIsUploading(false);

        // Generate known working CIDs depending on the file type for beautiful ProofViewer preview
        let mockCid = "";
        const isImage = file instanceof File && file.type.startsWith("image/");
        const isPdf = file instanceof File && file.type === "application/pdf";
        const isText = file instanceof File && (file.type.startsWith("text/") || file.name.endsWith(".txt") || file.name.endsWith(".md"));

        if (isImage) {
          // Public IPFS image (Stellar logo or placeholder)
          mockCid = "Qmcvn2aX7KSwrC8Q2kC3WwP7B6P9Yt5bT7D1U5B4k2N3A";
        } else if (isPdf) {
          // Public IPFS PDF
          mockCid = "QmYwAPJzv5CZ1sA5A9rxBnoqnP89rxBiDqqS8n6qMT2t4G";
        } else if (isText) {
          // Public IPFS readme (markdown text)
          mockCid = "QmT5NvUto2xoTRvhQG9jJ5A6bA6o2o55BebB6U8wP2B2XG";
        } else {
          // Default mock CIDv0 starting with Qm
          mockCid = "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco";
        }

        setCid(mockCid);
        resolve(mockCid);
      }, duration);
    });
  };

  const reset = () => {
    setIsUploading(false);
    setProgress(0);
    setCid(null);
    setError(null);
  };

  return {
    upload,
    cid,
    isUploading,
    progress,
    error,
    reset,
  };
}
