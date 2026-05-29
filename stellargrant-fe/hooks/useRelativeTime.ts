"use client";

import { useEffect, useState } from "react";

function formatRelative(date: Date, now = Date.now()): string {
  const diffSec = Math.round((date.getTime() - now) / 1000);
  const abs = Math.abs(diffSec);
  const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

  if (abs < 60) return rtf.format(diffSec, "second");
  if (abs < 3600) return rtf.format(Math.round(diffSec / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(diffSec / 3600), "hour");
  if (abs < 604800) return rtf.format(Math.round(diffSec / 86400), "day");
  return date.toLocaleDateString();
}

export function useRelativeTime(date: Date | string | number): string {
  const [label, setLabel] = useState(() =>
    formatRelative(new Date(date))
  );

  useEffect(() => {
    const target = new Date(date);
    const tick = () => setLabel(formatRelative(target));
    tick();
    const id = setInterval(tick, 60_000);
    return () => clearInterval(id);
  }, [date]);

  return label;
}
