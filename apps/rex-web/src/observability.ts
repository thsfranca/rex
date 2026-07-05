import type { PendingApproval, TurnPhase } from "./types";

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
  enabled: boolean;
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
