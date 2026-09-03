"use client";

import { useState, useCallback } from "react";

const PINATA_JWT = process.env.NEXT_PUBLIC_PINATA_JWT ?? "";
const PINATA_URL = "https://api.pinata.cloud/pinning/pinFileToIPFS";

function generateMockCid(file?: File): string {
  if (file?.type.startsWith("image/")) return "Qmcvn2aX7KSwrC8Q2kC3WwP7B6P9Yt5bT7D1U5B4k2N3A";
  if (file?.type === "application/pdf") return "QmYwAPJzv5CZ1sA5A9rxBnoqnP89rxBiDqqS8n6qMT2t4G";
  if (file && (file.type.startsWith("text/") || file.name.endsWith(".md") || file.name.endsWith(".txt"))) {
    return "QmT5NvUto2xoTRvhQG9jJ5A6bA6o2o55BebB6U8wP2B2XG";
  }
  return "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco";
}

async function mockUpload(onProgress: (pct: number) => void): Promise<string> {
  return new Promise((resolve) => {
    const duration = 1200;
    const step = 80;
    const steps = duration / step;
    let current = 0;
    const timer = setInterval(() => {
      current++;
      onProgress(Math.min(Math.round((current / steps) * 100), 95));
    }, step);
    setTimeout(() => {
      clearInterval(timer);
      onProgress(100);
      resolve("QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco");
    }, duration);
  });
}

function uploadFileViaPinata(
  formData: FormData,
  onProgress: (pct: number) => void
): Promise<string> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", PINATA_URL);
    xhr.setRequestHeader("Authorization", `Bearer ${PINATA_JWT}`);

    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable) {
        onProgress(Math.round((e.loaded / e.total) * 95));
      }
    };

    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        try {
          const json = JSON.parse(xhr.responseText) as { IpfsHash: string };
          onProgress(100);
          resolve(json.IpfsHash);
        } catch {
          reject(new Error("Invalid response from Pinata"));
        }
      } else {
        reject(new Error(`Pinata upload failed: ${xhr.status} ${xhr.statusText}`));
      }
    };

    xhr.onerror = () => reject(new Error("Network error during upload"));
    xhr.send(formData);
  });
}

export function useIPFS() {
  const [isUploading, setIsUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState(0);
  const [cid, setCid] = useState<string | null>(null);
  const [error, setError] = useState<Error | null>(null);

  const useMock = !PINATA_JWT;

  const upload = useCallback(async (file: File | object): Promise<string> => {
    setIsUploading(true);
    setUploadProgress(0);
    setError(null);
    setCid(null);

    try {
      let resultCid: string;

      if (useMock) {
        // Fallback mode: simulate upload with mock CID
        const mockFile = file instanceof File ? file : undefined;
        resultCid = await mockUpload(setUploadProgress);
        resultCid = mockFile ? generateMockCid(mockFile) : resultCid;
        setUploadProgress(100);
      } else if (file instanceof File) {
        const fd = new FormData();
        fd.append("file", file);
        fd.append(
          "pinataMetadata",
          JSON.stringify({ name: file.name })
        );
        resultCid = await uploadFileViaPinata(fd, setUploadProgress);
      } else {
        // Plain object → serialize to JSON and upload as metadata.json
        const blob = new Blob([JSON.stringify(file)], { type: "application/json" });
        const jsonFile = new File([blob], "metadata.json", { type: "application/json" });
        const fd = new FormData();
        fd.append("file", jsonFile);
        fd.append("pinataMetadata", JSON.stringify({ name: "metadata.json" }));
        resultCid = await uploadFileViaPinata(fd, setUploadProgress);
      }

      setCid(resultCid);
      return resultCid;
    } catch (err) {
      const e = err instanceof Error ? err : new Error(String(err));
      setError(e);
      throw e;
    } finally {
      setIsUploading(false);
    }
  }, [useMock]);

  const uploadText = useCallback(async (text: string, filename = "upload.md"): Promise<string> => {
    const blob = new Blob([text], { type: "text/plain" });
    const file = new File([blob], filename, { type: "text/plain" });
    return upload(file);
  }, [upload]);

  const reset = useCallback(() => {
    setIsUploading(false);
    setUploadProgress(0);
    setCid(null);
    setError(null);
  }, []);

  return {
    upload,
    uploadText,
    cid,
    isUploading,
    uploadProgress,
    error,
    reset,
  };
}
