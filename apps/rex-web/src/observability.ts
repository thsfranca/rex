import type { PendingApproval, TurnPhase } from "./types";

const STORAGE_KEY = "rexUiObservability";
const MAX_STREAM_EVENTS = 32;

export interface UiObservabilitySnapshot {
  enabled: boolean;
  phase: TurnPhase;
  statusLabel: string;
  pendingApproval: PendingApproval | null;
  error: string | null;
  harnessSessionId: string | null;
  lastSubmitError: string | null;
  composerBusy: boolean;
  streamEvents: string[];
  updatedAt: string;
}

export function isObservabilityEnabled(): boolean {
  if (typeof window === "undefined") return false;
  try {
    if (localStorage.getItem(STORAGE_KEY) === "1") return true;
  } catch {
    // Private browsing or disabled storage.
  }
  return new URLSearchParams(window.location.search).get("rex_ui_obs") === "1";
}

export function enableObservability(): void {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, "1");
  } catch {
    // Best-effort for harness injection.
  }
}

export function formatStreamEvent(event: unknown): string {
  try {
    return JSON.stringify(event);
  } catch {
    return String(event);
  }
}

export function trimStreamEvents(events: string[]): string[] {
  if (events.length <= MAX_STREAM_EVENTS) return events;
  return events.slice(events.length - MAX_STREAM_EVENTS);
}

export function buildObservabilitySnapshot(input: {
  phase: TurnPhase;
  statusLabel: string;
  pendingApproval: PendingApproval | null;
  error: string | null;
  harnessSessionId: string | null;
  lastSubmitError: string | null;
  composerBusy: boolean;
  streamEvents: string[];
}): UiObservabilitySnapshot {
  return {
    enabled: isObservabilityEnabled(),
    ...input,
    streamEvents: trimStreamEvents(input.streamEvents),
    updatedAt: new Date().toISOString(),
  };
}

export function publishObservabilitySnapshot(snapshot: UiObservabilitySnapshot): void {
  if (typeof window === "undefined") return;
  (window as RexUiObservabilityWindow).__REX_UI_OBSERVABILITY__ = snapshot;
}

interface RexUiObservabilityWindow extends Window {
  __REX_UI_OBSERVABILITY__?: UiObservabilitySnapshot;
}
