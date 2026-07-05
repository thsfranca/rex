import type { UiObservabilitySnapshot } from "../observability";

interface Props {
  snapshot: UiObservabilitySnapshot;
}

export function UiObservability({ snapshot }: Props) {
  const attrs = {
    "data-testid": "ui-observability",
    "data-phase": snapshot.phase,
    "data-status": snapshot.statusLabel,
    "data-pending-approval": snapshot.pendingApproval ? "yes" : "no",
    "data-error": snapshot.error ?? "",
    "data-submit-error": snapshot.lastSubmitError ?? "",
    "data-session-id": snapshot.harnessSessionId ?? "",
    "data-composer-busy": snapshot.composerBusy ? "yes" : "no",
    "data-stream-events": snapshot.streamEvents.length,
  };

  if (!snapshot.enabled) {
    return <aside hidden className="ui-observability" {...attrs} />;
  }

  return (
    <aside className="ui-observability" {...attrs}>
      <pre data-testid="ui-observability-body">{JSON.stringify(snapshot, null, 2)}</pre>
    </aside>
  );
}
