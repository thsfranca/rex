import type { StreamErrorCode, StreamErrorEvent } from "./ndjsonParser";

export interface ClassifiedStreamError {
  readonly code: StreamErrorCode;
  readonly message: string;
  readonly retryable: boolean;
}

export function classifyStreamError(error: StreamErrorEvent): ClassifiedStreamError {
  const fromEvent = error.code;
  if (fromEvent !== undefined) {
    return {
      code: fromEvent,
      message: error.message,
      retryable: fromEvent === "daemon_unavailable" || fromEvent === "stream_timeout",
    };
  }
  return classifyStreamErrorMessage(error.message);
}

export function classifyStreamErrorMessage(message: string): ClassifiedStreamError {
  const normalized = message.trim().toLowerCase();
  if (normalized === "cancelled") {
    return { code: "cancelled", message, retryable: true };
  }
  if (
    normalized.includes("daemon is unavailable") ||
    normalized.includes("daemon unavailable") ||
    normalized.includes("no such file")
  ) {
    return { code: "daemon_unavailable", message, retryable: true };
  }
  if (normalized.includes("timed out")) {
    return { code: "stream_timeout", message, retryable: true };
  }
  if (normalized.includes("interrupted")) {
    return { code: "stream_interrupted", message, retryable: true };
  }
  if (normalized.includes("without completion marker")) {
    return { code: "stream_incomplete", message, retryable: false };
  }
  if (normalized.includes("malformed ndjson") || normalized.includes("unknown ndjson event type")) {
    return { code: "invalid_response", message, retryable: false };
  }
  if (normalized.includes("failed to spawn rex-cli")) {
    return { code: "spawn_failed", message, retryable: false };
  }
  return { code: "unknown", message, retryable: false };
}
