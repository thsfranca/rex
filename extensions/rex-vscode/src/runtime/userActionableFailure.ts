import type { StreamErrorCode } from "./ndjsonParser";

const SETUP_HINT_CODES: ReadonlySet<StreamErrorCode> = new Set([
  "daemon_unavailable",
  "spawn_failed",
  "stream_timeout",
]);

/**
 * True when a terminal stream error is likely fixed by settings (absolute CLI path)
 * or by starting the daemon / checking output logs.
 */
export function streamFailureWantsSetupHint(code: StreamErrorCode): boolean {
  return SETUP_HINT_CODES.has(code);
}
