/**
 * Bounded string growth for stderr, status UI, and similar paths (long sessions).
 */
const TRUNC_MARKER = "[rex: stderr truncated]";

export function elideForTooltip(value: string, maxChars: number): string {
  if (value.length <= maxChars) {
    return value;
  }
  if (maxChars < 2) {
    return "…";
  }
  return `${value.slice(0, maxChars - 1)}…`;
}

export function appendWithByteCap(current: string, add: string, maxBytes: number): string {
  const next = current + add;
  if (next.length <= maxBytes) {
    return next;
  }
  const headChars = Math.min(12_000, Math.floor(maxBytes / 2) - TRUNC_MARKER.length);
  const tailChars = Math.max(0, maxBytes - headChars - TRUNC_MARKER.length);
  if (headChars < 256 || tailChars < 256) {
    return next.slice(0, maxBytes);
  }
  const head = next.slice(0, headChars);
  const tail = next.slice(-tailChars);
  return `${head}${TRUNC_MARKER}${tail}`;
}
