import type { StreamErrorCode } from "./ndjsonParser";

const SETUP_HINT_CODES: ReadonlySet<StreamErrorCode> = new Set([
  "daemon_unavailable",
  "spawn_failed",
  "stream_timeout",
]);

const SETUP_HINT_MESSAGE_MARKERS: readonly string[] = [
  "REX_SIDECAR",
  "rex-sidecar-stub",
  "sidecar",
  "approval gate",
  "approval checkpoint",
  "checkpoint required",
  "REX_AGENT_APPROVALS",
  "REX_OPENAI_COMPAT",
];

/**
 * True when a terminal stream error is likely fixed by settings (absolute CLI path),
 * daemon/sidecar env, brokered HTTP, or approval configuration.
 */
export function streamFailureWantsSetupHint(
  code: StreamErrorCode,
  message?: string,
): boolean {
  if (SETUP_HINT_CODES.has(code)) {
    return true;
  }
  return messageIndicatesSetupHint(message);
}

function messageIndicatesSetupHint(message: string | undefined): boolean {
  if (message === undefined || message.trim().length === 0) {
    return false;
  }
  const lower = message.toLowerCase();
  return SETUP_HINT_MESSAGE_MARKERS.some((marker) => lower.includes(marker.toLowerCase()));
}
