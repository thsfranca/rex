import type { UiObservabilitySnapshot } from "../observability";

interface Props {
  snapshot: UiObservabilitySnapshot;
}

export function UiObservability({ snapshot }: Props) {
  const pending = snapshot.pendingApproval
    ? `${snapshot.pendingApproval.toolName}:${snapshot.pendingApproval.approvalToken ? "token" : "no-token"}`
    : "none";

  return (
    <aside
      className="ui-observability"
      data-testid="ui-observability"
      data-phase={snapshot.phase}
      data-status={snapshot.statusLabel}
      data-pending-approval={snapshot.pendingApproval ? "yes" : "no"}
      data-error={snapshot.error ?? ""}
      data-submit-error={snapshot.lastSubmitError ?? ""}
      data-session-id={snapshot.harnessSessionId ?? ""}
      data-composer-busy={snapshot.composerBusy ? "yes" : "no"}
      data-stream-events={snapshot.streamEvents.length}
      aria-hidden={!snapshot.enabled}
    >
      {snapshot.enabled ? (
        <pre data-testid="ui-observability-body">{JSON.stringify(snapshot, null, 2)}</pre>
      ) : (
        <span data-testid="ui-observability-summary">
          {snapshot.phase}|pending={pending}|err={snapshot.error ?? snapshot.lastSubmitError ?? "none"}
        </span>
      )}
    </aside>
  );
}
