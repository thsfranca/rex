/**
 * Bounded string growth for stderr and similar capture paths (long sessions).
 */
const TRUNC_MARKER = "[rex: stderr truncated]";

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
