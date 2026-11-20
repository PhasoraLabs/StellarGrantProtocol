"use client";

import { useRef, useEffect } from "react";
import QRCodeLib from "qrcode";

export interface QRCodeProps {
  value: string;
  size?: number;
  label?: string;
  downloadable?: boolean;
}

export function QRCode({
  value,
  size = 200,
  label,
  downloadable = false,
}: QRCodeProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvasRef.current || !value) return;
    QRCodeLib.toCanvas(canvasRef.current, value, {
      width: size,
      color: { dark: "#E8EDF5", light: "#111D35" },
      errorCorrectionLevel: "M",
    }).catch(() => {
      // canvas render failure — silently ignore
    });
  }, [value, size]);

  const download = () => {
    const url = canvasRef.current?.toDataURL("image/png");
    if (!url) return;
    const a = document.createElement("a");
    a.href = url;
    a.download = "stellar-address-qr.png";
    a.click();
  };

  return (
    <div className="flex flex-col items-center gap-3">
      <div className="bg-surface p-3 border border-border-color rounded-none">
        <canvas ref={canvasRef} />
      </div>
      {label && (
        <p className="font-mono text-xs text-text-muted">{label}</p>
      )}
      {downloadable && (
        <button
          type="button"
          onClick={download}
          className="font-mono text-xs text-accent-secondary hover:underline"
        >
          Download QR image
        </button>
      )}
    </div>
  );
}
