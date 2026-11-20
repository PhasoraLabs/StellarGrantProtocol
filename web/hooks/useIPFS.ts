"use client";

import { useState, useCallback } from "react";

function uploadFileViaApi(
  formData: FormData,
  onProgress: (pct: number) => void
): Promise<string> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", "/api/ipfs");

    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable) {
        onProgress(Math.round((e.loaded / e.total) * 95));
      }
    };

    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        try {
          const json = JSON.parse(xhr.responseText) as { IpfsHash?: string };
          if (!json.IpfsHash) throw new Error("Upload response did not include a CID");
          onProgress(100);
          resolve(json.IpfsHash);
        } catch {
          reject(new Error("Invalid response from IPFS upload endpoint"));
        }
      } else {
        let message = `IPFS upload failed: ${xhr.status} ${xhr.statusText}`.trim();
        try {
          const json = JSON.parse(xhr.responseText) as { error?: string };
          if (json.error) message = json.error;
        } catch {
          // Keep the status-based message for non-JSON responses.
        }
        reject(new Error(message));
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

  const upload = useCallback(async (file: File | object): Promise<string> => {
    setIsUploading(true);
    setUploadProgress(0);
    setError(null);
    setCid(null);

    try {
      let resultCid: string;

      if (file instanceof File) {
        const fd = new FormData();
        fd.append("file", file);
        fd.append(
          "pinataMetadata",
          JSON.stringify({ name: file.name })
        );
        resultCid = await uploadFileViaApi(fd, setUploadProgress);
      } else {
        // Plain object → serialize to JSON and upload as metadata.json
        const blob = new Blob([JSON.stringify(file)], { type: "application/json" });
        const jsonFile = new File([blob], "metadata.json", { type: "application/json" });
        const fd = new FormData();
        fd.append("file", jsonFile);
        fd.append("pinataMetadata", JSON.stringify({ name: "metadata.json" }));
        resultCid = await uploadFileViaApi(fd, setUploadProgress);
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
  }, []);

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
